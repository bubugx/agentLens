use crate::{collector::now, model::*};
use rusqlite::{params, Connection};
use std::{collections::HashSet, fs, path::Path};
pub struct Store {
    conn: Connection,
}
impl Store {
    pub fn open(path: &Path) -> Result<Self, String> {
        fs::create_dir_all(path).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o700))
                .map_err(|e| e.to_string())?;
        }
        let conn = Connection::open(path.join("agentlens.sqlite")).map_err(|e| e.to_string())?;
        conn.busy_timeout(std::time::Duration::from_secs(3))
            .map_err(|e| e.to_string())?;
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if version > 1 {
            return Err("History database was created by a newer AgentLens version; the file has been preserved".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(
                path.join("agentlens.sqlite"),
                fs::Permissions::from_mode(0o600),
            )
            .map_err(|e| e.to_string())?;
        }
        conn.execute_batch(
            r#"
                PRAGMA user_version=1;
                PRAGMA journal_mode=WAL;
                PRAGMA synchronous=FULL;
                CREATE TABLE IF NOT EXISTS config (
                    key TEXT PRIMARY KEY, value TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS history (
                    time INTEGER PRIMARY KEY, data TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS activity (
                    id TEXT PRIMARY KEY, time INTEGER NOT NULL, data TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS inventory_files (
                    path TEXT PRIMARY KEY,
                    root_id TEXT NOT NULL,
                    status TEXT NOT NULL,
                    size INTEGER NOT NULL,
                    data TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS inventory_status
                    ON inventory_files(status, root_id, size DESC, path);
                CREATE TABLE IF NOT EXISTS inventory_roots (
                    id TEXT PRIMARY KEY, data TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS operations (
                    id TEXT PRIMARY KEY, time INTEGER NOT NULL,
                    state TEXT NOT NULL, data TEXT NOT NULL
                );
            "#,
        )
        .map_err(|e| e.to_string())?;
        conn.execute_batch(
            r#"
                CREATE TABLE IF NOT EXISTS resource_samples (
                    time INTEGER NOT NULL,
                    scope TEXT NOT NULL,
                    scope_id TEXT NOT NULL,
                    memory INTEGER NOT NULL,
                    cpu REAL NOT NULL,
                    processes INTEGER NOT NULL,
                    PRIMARY KEY(scope, scope_id, time)
                );
                CREATE INDEX IF NOT EXISTS resource_samples_time ON resource_samples(time);
            "#,
        )
        .map_err(|e| e.to_string())?;
        Ok(Self { conn })
    }
    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.conn
            .query_row("SELECT value FROM config WHERE key=?", [key], |r| {
                r.get::<_, String>(0)
            })
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
    }
    pub fn set<T: serde::Serialize>(&self, key: &str, value: &T) -> Result<(), String> {
        let json = serde_json::to_string(value).map_err(|e| e.to_string())?;
        self.conn
            .execute(
                "INSERT OR REPLACE INTO config VALUES (?,?)",
                params![key, json],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    /// Registry changes, bound roots and stale inventory removal commit together.
    pub fn save_registry(
        &mut self,
        definitions: &[crate::adoption::AgentDefinition],
        roots: &[Root],
    ) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        for (key, value) in [
            ("agent_definitions", serde_json::to_string(definitions)),
            (
                "roots",
                serde_json::to_string(&roots.iter().filter(|r| r.custom).collect::<Vec<_>>()),
            ),
        ] {
            tx.execute(
                "INSERT OR REPLACE INTO config VALUES (?,?)",
                params![key, value.map_err(|e| e.to_string())?],
            )
            .map_err(|e| e.to_string())?;
        }
        let ids: HashSet<_> = roots.iter().map(|r| r.id.as_str()).collect();
        let old = {
            let mut stmt = tx
                .prepare("SELECT id FROM inventory_roots")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |r| r.get::<_, String>(0))
                .map_err(|e| e.to_string())?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?
        };
        for id in old.iter().filter(|id| !ids.contains(id.as_str())) {
            tx.execute("DELETE FROM inventory_files WHERE root_id=?", [id])
                .map_err(|e| e.to_string())?;
            tx.execute("DELETE FROM inventory_roots WHERE id=?", [id])
                .map_err(|e| e.to_string())?;
        }
        tx.commit().map_err(|e| e.to_string())
    }
    #[cfg(test)]
    pub fn replace_inventory(&mut self, files: &[FileEntry], roots: &[Root]) -> Result<(), String> {
        self.replace_inventory_checked(files, roots, &|| false)
            .map(|_| ())
    }
    pub fn replace_inventory_checked(
        &mut self,
        files: &[FileEntry],
        roots: &[Root],
        cancelled: &dyn Fn() -> bool,
    ) -> Result<bool, String> {
        let transaction = self.conn.transaction().map_err(|e| e.to_string())?;
        // Reconcile in one transaction; unchanged rows never rewrite the database
        // or its secondary index. Keep only paths in memory to find removed files.
        let mut removed: HashSet<String> = transaction
            .prepare("SELECT path FROM inventory_files")
            .map_err(|e| e.to_string())?
            .query_map([], |r| r.get(0))
            .map_err(|e| e.to_string())?
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?;
        let mut seen = HashSet::with_capacity(files.len());
        {
            let mut insert = transaction
                .prepare(
                    r#"
                INSERT INTO inventory_files VALUES (?, ?, ?, ?, ?)
                ON CONFLICT(path) DO UPDATE SET
                    root_id=excluded.root_id,
                    status=excluded.status,
                    size=excluded.size,
                    data=excluded.data
                WHERE inventory_files.data<>excluded.data
            "#,
                )
                .map_err(|e| e.to_string())?;
            for (index, file) in files.iter().enumerate() {
                if index % 256 == 0 && cancelled() {
                    return Ok(false);
                }
                if !seen.insert(&file.path) {
                    return Err("Duplicate inventory path".into());
                }
                removed.remove(&file.path);
                insert
                    .execute(params![
                        file.path,
                        file.root_id,
                        file.cleanup_status,
                        file.size,
                        serde_json::to_string(file).map_err(|e| e.to_string())?
                    ])
                    .map_err(|e| e.to_string())?;
            }
            let mut delete = transaction
                .prepare("DELETE FROM inventory_files WHERE path=?")
                .map_err(|e| e.to_string())?;
            for path in removed {
                if cancelled() {
                    return Ok(false);
                }
                delete.execute([path]).map_err(|e| e.to_string())?;
            }
            transaction
                .execute("DELETE FROM inventory_roots", [])
                .map_err(|e| e.to_string())?;
            let mut insert = transaction
                .prepare("INSERT INTO inventory_roots VALUES (?,?)")
                .map_err(|e| e.to_string())?;
            for root in roots {
                insert
                    .execute(params![
                        root.id,
                        serde_json::to_string(root).map_err(|e| e.to_string())?
                    ])
                    .map_err(|e| e.to_string())?;
            }
        }
        if cancelled() {
            return Ok(false);
        }
        transaction
            .commit()
            .map(|_| true)
            .map_err(|e| e.to_string())
    }
    pub fn inventory_details(
        &self,
        status: &str,
        root: Option<&str>,
        query: &str,
        offset: usize,
    ) -> Result<InventoryDetails, String> {
        let read = self
            .conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        let pattern = format!(
            "%{}%",
            query
                .replace('!', "!!")
                .replace('%', "!%")
                .replace('_', "!_")
        );
        let filter = "status=?1 AND path LIKE ?2 ESCAPE '!'";
        let mut groups = Vec::new();
        let mut statement = read
            .prepare(&format!(
                r#"
                SELECT root_id, COUNT(*), SUM(size), r.data
                FROM inventory_files f
                LEFT JOIN inventory_roots r ON r.id=f.root_id
                WHERE {filter}
                GROUP BY root_id
                ORDER BY SUM(size) DESC, root_id
            "#
            ))
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map(params![status, pattern], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, usize>(1)?,
                    r.get::<_, u64>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let (id, count, bytes, json) = row.map_err(|e| e.to_string())?;
            let r: Root = serde_json::from_str(&json).map_err(|e| e.to_string())?;
            groups.push(InventoryGroup {
                id,
                count,
                bytes,
                label: r.label,
                path: r.path,
                agent: r.agent,
                project: r.custom,
            });
        }
        let count = groups
            .iter()
            .filter(|g| root.is_none_or(|r| g.id == r))
            .map(|g| g.count)
            .sum();
        let bytes = groups
            .iter()
            .filter(|g| root.is_none_or(|r| g.id == r))
            .map(|g| g.bytes)
            .sum();
        let mut statement = read
            .prepare(&format!(
                r#"
                SELECT data FROM inventory_files
                WHERE {filter} AND (?3 IS NULL OR root_id=?3)
                ORDER BY size DESC, path
                LIMIT 50 OFFSET ?4
            "#
            ))
            .map_err(|e| e.to_string())?;
        let rows = statement
            .query_map(params![status, pattern, root, offset], |r| {
                r.get::<_, String>(0)
            })
            .map_err(|e| e.to_string())?;
        let files = rows
            .map(|r| {
                r.map_err(|e| e.to_string())
                    .and_then(|json| serde_json::from_str(&json).map_err(|e| e.to_string()))
            })
            .collect::<Result<Vec<FileEntry>, String>>()?;
        Ok(InventoryDetails {
            count,
            bytes,
            groups,
            files,
            offset,
            limit: 50,
        })
    }
    pub fn history(&self) -> Vec<HistoryPoint> {
        let Ok(mut q) = self.conn.prepare(
            r#"
                WITH ranked AS (
                    SELECT data, time,
                        ROW_NUMBER() OVER (ORDER BY time) AS n,
                        COUNT(*) OVER () AS total
                    FROM history
                )
                SELECT data FROM ranked
                WHERE n % MAX(1, (total-6+493)/494)=0 OR n>total-6
                ORDER BY time DESC
            "#,
        ) else {
            return vec![];
        };
        let Ok(rows) = q.query_map([], |r| r.get::<_, String>(0)) else {
            return vec![];
        };
        let mut out: Vec<_> = rows
            .flatten()
            .filter_map(|s| serde_json::from_str(&s).ok())
            .collect();
        out.reverse();
        out
    }
    pub fn resource_samples(
        &mut self,
        samples: &std::collections::HashMap<(String, String), crate::resources::ResourcePoint>,
    ) -> Result<(), String> {
        let tx = self.conn.transaction().map_err(|e| e.to_string())?;
        {
            let mut insert = tx
                .prepare("INSERT OR REPLACE INTO resource_samples VALUES (?,?,?,?,?,?)")
                .map_err(|e| e.to_string())?;
            for ((scope, id), p) in samples {
                insert
                    .execute(params![p.time, scope, id, p.memory, p.cpu, p.processes])
                    .map_err(|e| e.to_string())?;
            }
        }
        tx.execute(
            "DELETE FROM resource_samples WHERE time<?",
            [now().saturating_sub(86400)],
        )
        .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())
    }
    pub fn resource_history(
        &self,
        scope: &str,
        id: &str,
        since: u64,
    ) -> Result<Vec<crate::resources::ResourcePoint>, String> {
        let width = (now().saturating_sub(since) / 240).max(1);
        let mut query = self
            .conn
            .prepare(
                r#"
                WITH ranked AS (
                    SELECT time, memory, cpu, processes,
                        ROW_NUMBER() OVER (
                            PARTITION BY time/?4 ORDER BY time DESC
                        ) AS n
                    FROM resource_samples
                    WHERE scope=?1 AND scope_id=?2 AND time>=?3
                )
                SELECT time, memory, cpu, processes FROM ranked
                WHERE n=1
                ORDER BY time
            "#,
            )
            .map_err(|e| e.to_string())?;
        let rows = query
            .query_map(params![scope, id, since, width], |r| {
                Ok(crate::resources::ResourcePoint {
                    time: r.get(0)?,
                    memory: r.get(1)?,
                    cpu: r.get(2)?,
                    processes: r.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())
    }
    pub fn activity(&self) -> Vec<Activity> {
        let Ok(mut q) = self
            .conn
            .prepare(r#"
                WITH ranked AS (
                    SELECT data, time, rowid,
                        ROW_NUMBER() OVER (
                            PARTITION BY CASE
                                WHEN json_extract(data,'$.kind') IN ('growth','decrease') THEN 'storage'
                                ELSE json_extract(data,'$.kind')
                            END
                            ORDER BY time DESC, rowid DESC
                        ) AS rank
                    FROM activity
                    WHERE time >= strftime('%s','now') - 30*86400
                )
                SELECT data FROM ranked
                WHERE rank<=400
                ORDER BY time DESC, rowid DESC
            "#)
        else {
            return vec![];
        };
        let Ok(rows) = q.query_map([], |r| r.get::<_, String>(0)) else {
            return vec![];
        };
        rows.flatten()
            .filter_map(|s| serde_json::from_str(&s).ok())
            .collect()
    }
    pub fn point(&self, p: &HistoryPoint, retention: u64) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO history VALUES (?,?)",
                params![p.time, serde_json::to_string(p).unwrap()],
            )
            .map_err(|e| e.to_string())?;
        self.conn
            .execute(
                "DELETE FROM history WHERE time < ?",
                [now().saturating_sub(retention * 86400)],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn event(&self, a: &Activity) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO activity VALUES (?,?,?)",
                params![a.id, a.time, serde_json::to_string(a).unwrap()],
            )
            .map_err(|e| e.to_string())?;
        self.conn
            .execute(
                "DELETE FROM activity WHERE time < ?",
                [now().saturating_sub(30 * 86400)],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    pub fn journal(&self, id: &str, state: &str, data: &str) -> Result<(), String> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO operations VALUES (?,?,?,?)",
                params![id, now(), state, data],
            )
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    pub fn interrupted(&self) -> bool {
        self.conn
            .query_row(
                "SELECT COUNT(*) FROM operations WHERE state='intent'",
                [],
                |r| r.get::<_, i64>(0),
            )
            .unwrap_or(0)
            > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preferences_and_history_survive_restart_and_are_bounded() {
        let temp = tempfile::tempdir().unwrap();
        {
            let store = Store::open(temp.path()).unwrap();
            let settings = Settings {
                interval: 10,
                ..Settings::default()
            };
            store.set("settings", &settings).unwrap();
            for i in 0..620 {
                let p = HistoryPoint {
                    time: now() - 620 * 60 + i * 60,
                    storage: i,
                    memory: i,
                    cpu: 0.0,
                    agents: vec![],
                };
                store.point(&p, 7).unwrap();
            }
        }
        let reopened = Store::open(temp.path()).unwrap();
        assert_eq!(reopened.get::<Settings>("settings").unwrap().interval, 10);
        let points = reopened.history();
        assert!(points.len() <= 500);
        assert_eq!(points.last().unwrap().storage, 619);
        assert_eq!(points[points.len() - 6].storage, 614);
    }
    #[test]
    fn newer_schema_is_preserved() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("agentlens.sqlite");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch("PRAGMA user_version=99").unwrap();
        drop(conn);
        assert!(Store::open(temp.path()).is_err());
        let conn = Connection::open(&path).unwrap();
        assert_eq!(
            conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            99
        );
    }
}

#[cfg(test)]
mod inventory_tests {
    use super::*;
    #[test]
    fn inventory_distribution_matches_full_totals_and_pages_across_restart() {
        let directory = tempfile::tempdir().unwrap();
        let roots: Vec<_> = (0..2)
            .map(|i| Root {
                id: i.to_string(),
                path: format!("/projects/{i}"),
                agent: "unknown".into(),
                label: format!("project-{i}"),
                custom: true,
                source: "scan".into(),
            })
            .collect();
        let files: Vec<_> = (0..132)
            .map(|i| FileEntry {
                id: format!("/projects/{}/file-{i}", i % 2),
                root_id: (i % 2).to_string(),
                path: format!("/projects/{}/file-{i}", i % 2),
                relative_path: format!("file-{i}"),
                agent: "unknown".into(),
                category: "project".into(),
                size: ((i + 1) * 10) as u64,
                modified: now(),
                identity: i.to_string(),
                eligible: false,
                reason: "Protected".into(),
                cleanup_status: if i < 130 { "protected" } else { "sessions" }.into(),
                session_id: None,
            })
            .collect();
        let mut store = Store::open(directory.path()).unwrap();
        store.replace_inventory(&files, &roots).unwrap();
        drop(store);
        let reopened = Store::open(directory.path()).unwrap();
        let all = reopened
            .inventory_details("protected", None, "", 0)
            .unwrap();
        assert_eq!(all.count, 130);
        assert_eq!(all.bytes, files[..130].iter().map(|f| f.size).sum::<u64>());
        assert_eq!(all.groups.iter().map(|g| g.bytes).sum::<u64>(), all.bytes);
        assert_eq!(all.groups.len(), 2);
        let second = reopened
            .inventory_details("protected", None, "", 50)
            .unwrap();
        let third = reopened
            .inventory_details("protected", None, "", 100)
            .unwrap();
        let ids: std::collections::HashSet<_> = all
            .files
            .iter()
            .chain(&second.files)
            .chain(&third.files)
            .map(|f| f.id.as_str())
            .collect();
        assert_eq!(ids.len(), 130);
        let project = reopened
            .inventory_details("protected", Some("1"), "", 0)
            .unwrap();
        assert_eq!(project.count, 65);
        assert!(project.files.iter().all(|f| f.root_id == "1"));
        assert!(reopened
            .inventory_details("protected", None, "%", 0)
            .unwrap()
            .files
            .is_empty());
        assert_eq!(
            reopened
                .inventory_details("protected", None, "file-129", 0)
                .unwrap()
                .count,
            1
        );
    }
    #[test]
    fn inventory_reconciliation_skips_unchanged_rows_and_removes_stale_files() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = Store::open(directory.path()).unwrap();
        let root = Root {
            id: "p".into(),
            path: "/p".into(),
            label: "p".into(),
            agent: "codex".into(),
            custom: false,
            source: "global".into(),
        };
        let mut file = FileEntry {
            id: "/p/f".into(),
            root_id: "p".into(),
            path: "/p/f".into(),
            relative_path: "f".into(),
            agent: "codex".into(),
            category: "cache".into(),
            size: 10,
            modified: now(),
            identity: "1".into(),
            eligible: false,
            reason: "retained".into(),
            cleanup_status: "recent".into(),
            session_id: None,
        };
        let mut other = file.clone();
        other.id = "/p/g".into();
        other.path = other.id.clone();
        store
            .replace_inventory(&[file.clone(), other.clone()], std::slice::from_ref(&root))
            .unwrap();
        store
            .conn
            .execute_batch(
                r#"
                CREATE TEMP TABLE updates (n INTEGER);
                INSERT INTO updates VALUES (0);
                CREATE TEMP TRIGGER count_updates AFTER UPDATE ON inventory_files
                BEGIN
                    UPDATE updates SET n=n+1;
                END;
            "#,
            )
            .unwrap();
        store
            .replace_inventory(&[file.clone(), other], std::slice::from_ref(&root))
            .unwrap();
        assert_eq!(
            store
                .conn
                .query_row("SELECT n FROM updates", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            0
        );
        file.size = 99;
        store.replace_inventory(&[file], &[root]).unwrap();
        assert_eq!(
            store
                .conn
                .query_row("SELECT n FROM updates", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            1
        );
        let details = store.inventory_details("recent", None, "", 0).unwrap();
        assert_eq!((details.count, details.bytes), (1, 99));
    }
    #[test]
    fn failed_inventory_transaction_keeps_previous_index() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = Store::open(directory.path()).unwrap();
        let roots = vec![Root {
            id: "p".into(),
            path: "/p".into(),
            label: "p".into(),
            agent: "unknown".into(),
            custom: true,
            source: "scan".into(),
        }];
        let f = FileEntry {
            id: "/p/f".into(),
            root_id: "p".into(),
            path: "/p/f".into(),
            relative_path: "f".into(),
            agent: "unknown".into(),
            category: "project".into(),
            size: 10,
            modified: now(),
            identity: "1".into(),
            eligible: false,
            reason: "protected".into(),
            cleanup_status: "protected".into(),
            session_id: None,
        };
        store.replace_inventory(&[f.clone()], &roots).unwrap();
        assert!(store.replace_inventory(&[f.clone(), f], &roots).is_err());
        assert_eq!(
            store
                .inventory_details("protected", None, "", 0)
                .unwrap()
                .bytes,
            10
        );
    }
}

#[cfg(test)]
mod resource_activity_tests {
    use super::*;
    #[test]
    fn resource_history_persists_exact_scopes_and_bounds_retention() {
        let directory = tempfile::tempdir().unwrap();
        let mut store = Store::open(directory.path()).unwrap();
        let time = now();
        for (time, memory) in [(time - 90000, 900), (time - 720, 100), (time, 200)] {
            let points = std::collections::HashMap::from([(
                ("process".into(), "1:10".into()),
                crate::resources::ResourcePoint {
                    time,
                    memory,
                    cpu: 2.5,
                    processes: 1,
                },
            )]);
            store.resource_samples(&points).unwrap();
        }
        drop(store);
        let store = Store::open(directory.path()).unwrap();
        let history = store
            .resource_history("process", "1:10", time - 86400)
            .unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].memory, 100);
        assert_eq!(history[1].memory, 200);
        assert_eq!(history[1].cpu, 2.5);
        assert!(store
            .resource_history("process", "1:20", time - 86400)
            .unwrap()
            .is_empty());
        assert_eq!(
            store
                .conn
                .query_row("SELECT COUNT(*) FROM resource_samples", [], |r| r
                    .get::<_, usize>(0))
                .unwrap(),
            2
        );
    }
    #[test]
    fn old_operations_remain_readable_when_storage_records_exceed_global_limit() {
        let directory = tempfile::tempdir().unwrap();
        let store = Store::open(directory.path()).unwrap();
        let time = now();
        for (kind, count) in [
            ("cleanup", 1),
            ("discovery", 1),
            ("process", 1),
            ("growth", 450),
        ] {
            for i in 0..count {
                store
                    .event(&Activity {
                        id: format!("{kind}-{i}"),
                        time: time - i,
                        kind: kind.into(),
                        agent: "codex".into(),
                        title: "fixture".into(),
                        detail: "fixture".into(),
                        bytes: 0,
                        context: None,
                    })
                    .unwrap();
            }
        }
        let events = store.activity();
        assert_eq!(events.len(), 403);
        for kind in ["cleanup", "discovery", "process"] {
            assert!(events.iter().any(|e| e.kind == kind));
        }
    }
}
