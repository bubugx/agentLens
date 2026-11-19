use crate::adoption::{self, AgentBinding, AgentDefinition, ProcessCandidate};
use crate::model::*;
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use walkdir::WalkDir;

pub const AGENTS: &[(&str, &str)] = &[
    ("claude", "Claude Code"),
    ("codex", "Codex"),
    ("cursor", "Cursor"),
    ("gemini", "Gemini CLI"),
    ("opencode", "OpenCode"),
    ("aider", "Aider"),
    ("unknown", "Unassigned projects"),
];
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub fn discover() -> Vec<Root> {
    let Some(home) = dirs::home_dir() else {
        return vec![];
    };
    let mut specs = vec![
        ("codex", home.join(".codex")),
        ("claude", home.join(".claude")),
        ("cursor", home.join(".cursor")),
        ("gemini", home.join(".gemini")),
        ("opencode", home.join(".local/share/opencode")),
        ("aider", home.join(".aider")),
    ];
    if let Some(p) = std::env::var_os("CODEX_HOME") {
        specs.retain(|(a, _)| *a != "codex");
        specs.push(("codex", PathBuf::from(p)));
    }
    if let Some(p) = dirs::cache_dir() {
        for (agent, names) in [
            ("opencode", vec!["opencode"]),
            ("codex", vec!["com.openai.codex", "Codex"]),
            ("claude", vec!["claude", "claude-cli"]),
            ("cursor", vec!["Cursor", "com.todesktop.230313mzl4w4u92"]),
        ] {
            for name in names {
                specs.push((agent, p.join(name)));
            }
        }
    }
    if let Some(p) = dirs::config_dir() {
        for name in ["Cache", "Code Cache", "GPUCache", "logs", "CachedData"] {
            specs.push(("cursor", p.join("Cursor").join(name)));
        }
    }
    #[cfg(target_os = "macos")]
    specs.push((
        "cursor",
        home.join("Library/Application Support/Cursor/User/workspaceStorage"),
    ));
    #[cfg(target_os = "windows")]
    if let Some(p) = dirs::config_dir() {
        specs.push(("cursor", p.join("Cursor/User/workspaceStorage")));
    }
    let mut roots = vec![];
    for (agent, path) in specs {
        if path.is_dir() && !path.is_symlink() {
            let Ok(path) = fs::canonicalize(path) else {
                continue;
            };
            let text = path.to_string_lossy().to_string();
            if roots.iter().any(|r: &Root| path.starts_with(&r.path)) {
                continue;
            }
            roots.push(Root {
                id: text.clone(),
                path: text,
                agent: agent.into(),
                label: format!(
                    "{} storage",
                    AGENTS.iter().find(|(a, _)| *a == agent).unwrap().1
                ),
                custom: false,
                source: "global".into(),
            });
        }
    }
    roots
}
pub fn fingerprint(_path: &Path, meta: &fs::Metadata) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            meta.dev(),
            meta.ino(),
            meta.len(),
            meta.mtime(),
            meta.mtime_nsec(),
            meta.ctime(),
            meta.ctime_nsec(),
            meta.nlink()
        )
    }
    #[cfg(not(unix))]
    {
        format!(
            "{:?}:{}:{:?}:{:?}",
            windows_identity(_path),
            meta.len(),
            meta.modified(),
            meta.created()
        )
    }
}
pub fn links_safe(_path: &Path, meta: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        meta.nlink() == 1
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        windows_identity(_path).is_some_and(|(_, links)| links == 1)
    }
}
pub fn classify(relative: &Path, root: &Root) -> (&'static str, Option<u64>) {
    let parts: Vec<_> = relative
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_lowercase())
        .collect();
    let first = parts.first().map(String::as_str).unwrap_or("");
    if root.custom && !root.source.starts_with("adopted-") {
        if parts.iter().any(|x| {
            matches!(
                x.as_str(),
                ".codex" | ".claude" | ".cursor" | ".gemini" | ".opencode" | ".aider"
            )
        }) || first.ends_with(".jsonl")
        {
            return ("sessions", None);
        }
        return ("project", None);
    }
    let root_parts: Vec<_> = Path::new(&root.path)
        .components()
        .map(|p| p.as_os_str().to_string_lossy().to_lowercase())
        .collect();
    if parts.iter().chain(root_parts.iter()).any(|part| {
        part.ends_with(".app")
            || part.ends_with(".framework")
            || part.ends_with(".bundle")
            || part == "org.sparkle-project.sparkle"
    }) {
        return ("application", None);
    }
    // Structured diagnostic logs use JSONL too. Exact log roots take precedence
    // over the generic conversation-extension fallback.
    let root_name = Path::new(&root.path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let log_scope = root.source == "adopted-logs"
        || root_name == "logs"
        || matches!(
            (root.agent.as_str(), first),
            ("codex" | "claude", "log" | "logs" | "debug")
        );
    if matches!(
        first,
        "sessions" | "archived_sessions" | "projects" | "history.jsonl"
    ) || (!log_scope && relative.extension().is_some_and(|x| x == "jsonl"))
    {
        return ("sessions", None);
    }
    // Only documented, exact directory components become candidates. Configuration / databases never do.
    let filename = relative
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    if filename.contains(".db")
        || filename.contains(".sqlite")
        || filename.contains("auth")
        || filename.contains("credential")
        || filename.contains("config")
        || filename.contains("secret")
        || filename.starts_with(".env")
        || matches!(
            relative.extension().and_then(|x| x.to_str()),
            Some("json" | "toml" | "yaml" | "yml" | "key" | "pem")
        )
    {
        return ("settings", None);
    }
    if log_scope
        && matches!(
            relative.extension().and_then(|x| x.to_str()),
            Some("log" | "jsonl" | "txt")
        )
    {
        return ("logs", Some(7 * 86400));
    }
    if (root.agent == "codex" && first == "shell_snapshots")
        || (root.agent == "claude" && first == "paste-cache")
    {
        return ("session_cache", Some(7 * 86400));
    }
    if matches!(
        relative.extension().and_then(|x| x.to_str()),
        Some(
            "rs" | "py"
                | "js"
                | "jsx"
                | "ts"
                | "tsx"
                | "vue"
                | "svelte"
                | "go"
                | "c"
                | "h"
                | "cpp"
                | "swift"
                | "java"
                | "rb"
                | "sh"
        )
    ) {
        return ("project", None);
    }
    if root.source.starts_with("adopted-") {
        return match root.source.as_str() {
            "adopted-cache" => ("cache", Some(7 * 86400)),
            "adopted-logs" => ("logs", Some(7 * 86400)),
            "adopted-temporary" => ("temporary", Some(86400)),
            _ => ("project", None),
        };
    }
    // Explicit app cache roots are scoped at discovery; children still pass
    // through the credential, source and database protection rules above.
    let root_path = Path::new(&root.path);
    let root_name = root_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let cache_root = dirs::cache_dir().is_some_and(|p| root_path.starts_with(p));
    if cache_root
        || matches!(
            root_name.as_str(),
            "cache" | "code cache" | "gpucache" | "cacheddata"
        )
    {
        return ("cache", Some(7 * 86400));
    }
    if root_name == "logs" {
        return ("logs", Some(7 * 86400));
    }
    match (root.agent.as_str(), first) {
        ("codex" | "claude" | "gemini", "tmp") => ("temporary", Some(86400)),
        ("codex" | "claude", "log" | "logs" | "debug") => ("logs", Some(7 * 86400)),
        (
            "codex" | "claude" | "gemini" | "opencode" | "cursor",
            "cache" | "Cache" | "code cache" | "gpucache",
        ) => ("cache", Some(7 * 86400)),
        _ if matches!(first, "settings.json" | "config.toml" | "auth.json") => ("settings", None),
        _ => ("other", None),
    }
}
pub fn cleanup_age(relative: &Path, root: &Root, policy: &CleanupPolicy) -> Option<u64> {
    let (category, rule) = classify(relative, root);
    if root.custom && !root.source.starts_with("adopted-") {
        if policy.include_project_sessions && policy.include_sessions {
            let parts: Vec<_> = relative
                .components()
                .map(|p| p.as_os_str().to_str().unwrap_or(""))
                .collect();
            if parts.len() >= 3
                && matches!(parts[0], ".codex" | ".claude")
                && matches!(parts[1], "sessions" | "archived_sessions" | "projects")
                && relative.extension().is_some_and(|e| e == "jsonl")
                && (policy.session_scope == "all" || parts[1] == "archived_sessions")
            {
                return Some(policy.session_days * 86400);
            }
        }
        return None;
    }
    match category {
        "temporary" if rule.is_some() && policy.temporary_enabled => {
            Some(policy.temporary_hours * 3600)
        }
        "cache" if rule.is_some() && policy.cache_enabled => Some(policy.cache_days * 86400),
        "logs" if rule.is_some() && policy.logs_enabled => Some(policy.log_days * 86400),
        "session_cache" if rule.is_some() && policy.session_cache_enabled => {
            Some(policy.session_cache_days * 86400)
        }
        "sessions"
            if policy.include_sessions
                && matches!(root.agent.as_str(), "codex" | "claude")
                && relative.extension().is_some_and(|e| e == "jsonl")
                && relative.components().next().is_some_and(|p| {
                    matches!(
                        p.as_os_str().to_str(),
                        Some("sessions" | "archived_sessions" | "projects")
                    )
                })
                && (policy.session_scope == "all" || relative.starts_with("archived_sessions")) =>
        {
            Some(policy.session_days * 86400)
        }
        _ => None,
    }
}
pub fn cleanup_exclusion(file: &FileEntry, policy: &CleanupPolicy) -> Option<&'static str> {
    if policy.excluded_agents.contains(&file.agent) {
        Some("This agent is excluded from cleanup")
    } else if policy
        .excluded_paths
        .iter()
        .any(|p| Path::new(&file.path).starts_with(p))
    {
        Some("This path is excluded from cleanup")
    } else if file.size < policy.minimum_size_mb * 1_000_000 {
        Some("Below the configured minimum file size")
    } else {
        None
    }
}
pub fn apply_session_retention(files: &mut [FileEntry], policy: &CleanupPolicy) {
    if !policy.include_sessions || policy.keep_latest_sessions == 0 {
        return;
    }
    let mut sessions: Vec<_> = files
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            f.category == "sessions" && Path::new(&f.path).extension().is_some_and(|e| e == "jsonl")
        })
        .map(|(i, _)| i)
        .collect();
    sessions.sort_by(|&a, &b| {
        files[b]
            .modified
            .cmp(&files[a].modified)
            .then_with(|| files[a].path.cmp(&files[b].path))
    });
    let mut counts = HashMap::<String, usize>::new();
    for i in sessions {
        let f = &mut files[i];
        let count = counts.entry(f.root_id.clone()).or_default();
        *count += 1;
        if *count <= policy.keep_latest_sessions {
            f.eligible = false;
            f.cleanup_status = "sessions".into();
            f.reason = "Retained among the newest session files in this directory".into();
        }
    }
}
pub fn discover_projects(
    home: &Path,
    roots: &[Root],
    sessions: &[Session],
    processes: &[ProcessEntry],
    ignored: &HashSet<String>,
) -> Vec<Root> {
    let mut result: Vec<Root> = Vec::new();
    let canonical_home = fs::canonicalize(home).unwrap_or_else(|_| home.to_path_buf());
    let candidates = sessions
        .iter()
        .filter(|s| s.confidence == "metadata")
        .map(|s| (s.project.as_str(), s.agent.as_str(), "session"))
        .chain(
            processes
                .iter()
                .map(|p| (p.working_directory.as_str(), p.agent.as_str(), "process")),
        );
    for (cwd, agent, source) in candidates {
        let path = Path::new(cwd);
        if !path.is_absolute() || path.is_symlink() || !path.is_dir() {
            continue;
        }
        let Ok(mut path) = fs::canonicalize(path) else {
            continue;
        };
        let Ok(relative) = path.strip_prefix(&canonical_home) else {
            continue;
        };
        let components: Vec<_> = relative.components().collect();
        // A project may live directly under the user's home directory (for
        // example ~/agentlens). Keep the home directory itself excluded, but
        // do not require an extra nesting level before discovering it.
        if components.is_empty()
            || components.first().is_some_and(|c| {
                matches!(
                    c.as_os_str().to_str(),
                    Some(
                        "Library"
                            | "AppData"
                            | ".codex"
                            | ".claude"
                            | ".cursor"
                            | ".cache"
                            | ".local"
                            | ".config"
                    )
                )
            })
        {
            continue;
        }
        // Prefer a repository root over a task's nested working directory.
        let mut parent = Some(path.as_path());
        while let Some(p) = parent {
            if p == canonical_home {
                break;
            }
            if p.join(".git").exists() {
                path = p.to_path_buf();
                break;
            }
            parent = p.parent();
        }
        if path == canonical_home
            || ignored.iter().any(|p| path.starts_with(p))
            || roots
                .iter()
                .chain(result.iter())
                .any(|r| path.starts_with(&r.path) || Path::new(&r.path).starts_with(&path))
        {
            continue;
        }
        let text = path.to_string_lossy().to_string();
        result.push(Root {
            id: text.clone(),
            path: text,
            agent: agent.into(),
            label: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            custom: true,
            source: source.into(),
        });
    }
    result
}
pub fn cleanup_summary(
    files: impl Iterator<Item = impl std::borrow::Borrow<FileEntry>>,
) -> Vec<CleanupBucket> {
    let mut buckets: HashMap<String, CleanupBucket> = HashMap::new();
    for file in files {
        let f = file.borrow();
        let bucket = buckets
            .entry(f.cleanup_status.clone())
            .or_insert_with(|| CleanupBucket {
                code: f.cleanup_status.clone(),
                ..Default::default()
            });
        bucket.count += 1;
        bucket.bytes += f.size;
    }
    let mut result: Vec<_> = buckets.into_values().collect();
    result.sort_by(|a, b| b.bytes.cmp(&a.bytes));
    result
}
pub fn path_is_safe(path: &Path, root: &Path) -> bool {
    if !path.starts_with(root) || path == root {
        return false;
    }
    let mut current = Some(path);
    while let Some(p) = current {
        let Ok(m) = fs::symlink_metadata(p) else {
            return false;
        };
        if m.is_symlink() {
            return false;
        }
        if p == root {
            return m.is_dir();
        }
        current = p.parent();
    }
    false
}
pub struct Scan {
    pub files: Vec<FileEntry>,
    pub sessions: Vec<Session>,
    pub warnings: Vec<String>,
    pub complete: bool,
}
#[cfg(test)]
pub fn scan(
    roots: &[Root],
    old: &HashMap<String, FileEntry>,
    active: &HashSet<String>,
    stable: &mut HashMap<String, (String, u64)>,
    policy: &CleanupPolicy,
) -> Scan {
    scan_with_progress(roots, old, active, stable, policy, &|_, _| true)
}
pub fn scan_with_progress(
    roots: &[Root],
    old: &HashMap<String, FileEntry>,
    active: &HashSet<String>,
    stable: &mut HashMap<String, (String, u64)>,
    policy: &CleanupPolicy,
    report: &dyn Fn(u64, &Path) -> bool,
) -> Scan {
    let mut out = Scan {
        files: vec![],
        sessions: vec![],
        warnings: vec![],
        complete: true,
    };
    let mut counted = HashSet::new();
    for root in roots {
        let base = Path::new(&root.path);
        if !base.is_dir() || base.is_symlink() {
            out.warnings
                .push(format!("Unavailable monitoring root: {}", root.label));
            out.complete = false;
            continue;
        }
        for entry in WalkDir::new(base)
            .follow_links(false)
            .same_file_system(true)
            .into_iter()
            .filter_entry(|e| {
                !(root.custom
                    && e.file_type().is_dir()
                    && [
                        "node_modules",
                        ".git",
                        "target",
                        ".next",
                        "dist",
                        "venv",
                        ".venv",
                    ]
                    .iter()
                    .any(|n| e.file_name() == *n))
            })
        {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => {
                    out.complete = false;
                    if !out.warnings.contains(&root.label) {
                        out.warnings.push(root.label.clone())
                    }
                    continue;
                }
            };
            if !entry.file_type().is_file() {
                continue;
            }
            let Ok(meta) = fs::symlink_metadata(entry.path()) else {
                out.complete = false;
                continue;
            };
            let path = entry.path().to_string_lossy().to_string();
            if !counted.insert(path.clone()) {
                continue;
            }
            if !report(meta.len(), entry.path()) {
                out.complete = false;
                break;
            }
            let relative = entry.path().strip_prefix(base).unwrap_or(entry.path());
            let (category, _) = classify(relative, root);
            let age_rule = cleanup_age(relative, root, policy);
            let modified = meta
                .modified()
                .unwrap_or(UNIX_EPOCH)
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let identity = fingerprint(entry.path(), &meta);
            let observation = stable
                .entry(path.clone())
                .or_insert_with(|| (identity.clone(), now()));
            if observation.0 != identity {
                *observation = (identity.clone(), now());
            }
            let stable_enough = now().saturating_sub(observation.1) >= 10;
            let old_enough = age_rule.is_some_and(|age| now().saturating_sub(modified) >= age);
            let eligible = old_enough
                && stable_enough
                && !active.contains(&root.agent)
                && links_safe(entry.path(), &meta);
            let category_disabled = match category {
                "temporary" => !policy.temporary_enabled,
                "cache" => !policy.cache_enabled,
                "logs" => !policy.logs_enabled,
                "session_cache" => !policy.session_cache_enabled,
                _ => false,
            };
            let (cleanup_status, reason) = if category_disabled {
                (
                    "excluded",
                    "This file category is disabled by cleanup rules",
                )
            } else if age_rule.is_none() && category == "sessions" {
                ("sessions", "Session history retained by policy")
            } else if age_rule.is_none() {
                (
                    "protected",
                    "Protected: configuration, application, source or unclassified data",
                )
            } else if !old_enough {
                ("recent", "Inside the configured retention window")
            } else if active.contains(&root.agent) {
                (
                    "active",
                    "Agent is running; close it before reviewing its files",
                )
            } else if !links_safe(entry.path(), &meta) {
                (
                    "unsafe",
                    "File identity or hard-link safety cannot be verified",
                )
            } else if !stable_enough {
                ("stability", "Waiting for a second stable observation")
            } else {
                (
                    "eligible",
                    "Inactive agent · stable file · outside configured retention window",
                )
            };
            let mut session_id = None;
            if category == "sessions" && relative.extension().is_some_and(|x| x == "jsonl") {
                if let Some(session) = read_session(entry.path(), &root.agent, meta.len(), modified)
                {
                    session_id = Some(session.id.clone());
                    out.sessions.push(session);
                }
            } else if let Some(prior) = old.get(&path) {
                session_id = prior.session_id.clone();
            }
            let mut file = FileEntry {
                id: path.clone(),
                root_id: root.id.clone(),
                path,
                relative_path: relative.to_string_lossy().to_string(),
                agent: root.agent.clone(),
                category: category.into(),
                size: meta.len(),
                modified,
                identity,
                eligible,
                reason: reason.into(),
                cleanup_status: cleanup_status.into(),
                session_id,
            };
            if age_rule.is_some() {
                if let Some(reason) = cleanup_exclusion(&file, policy) {
                    file.eligible = false;
                    file.cleanup_status = "excluded".into();
                    file.reason = reason.into();
                }
            }
            out.files.push(file);
        }
    }
    stable.retain(|p, _| counted.contains(p));
    out.files.sort_by(|a, b| b.size.cmp(&a.size));
    out.sessions.sort_by(|a, b| b.modified.cmp(&a.modified));
    out
}
fn read_session(path: &Path, agent: &str, size: u64, modified: u64) -> Option<Session> {
    // Read only bounded metadata records; never retain prompts, messages, tool output, or credentials.
    let reader = BufReader::new(fs::File::open(path).ok()?.take(64 * 1024));
    let mut budget = 64 * 1024;
    let mut id = None;
    let mut project = None;
    for line in reader.split(b'\n').take(16) {
        let line = line.ok()?;
        if line.len() > budget {
            break;
        }
        budget -= line.len();
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&line) {
            let payload = if value["type"] == "session_meta" {
                &value["payload"]
            } else {
                &value
            };
            if let Some(s) = payload["cwd"].as_str() {
                project = Some(s.to_string());
            }
            if let Some(s) = payload["sessionId"]
                .as_str()
                .or_else(|| payload["id"].as_str())
            {
                id = Some(s.to_string());
            }
            if project.is_some() && id.is_some() {
                break;
            }
        }
    }
    let confidence = if id.is_some() && project.is_some() {
        "metadata"
    } else {
        "directory"
    };
    Some(Session {
        id: id.unwrap_or_else(|| {
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        }),
        agent: agent.into(),
        project: project.unwrap_or_else(|| "Unknown project".into()),
        path: path.to_string_lossy().to_string(),
        size,
        modified,
        confidence: confidence.into(),
    })
}
pub(crate) fn identify(name: &str, exe: &str) -> Option<&'static str> {
    let name = name.to_lowercase();
    let exe = exe.to_lowercase();
    if matches!(name.as_str(), "claude" | "claude.exe") {
        Some("claude")
    } else if matches!(name.as_str(), "codex" | "codex.exe") {
        Some("codex")
    } else if name == "cursor"
        || name == "cursor.exe"
        || exe.contains("/cursor.app/")
        || exe.contains("\\cursor\\")
    {
        Some("cursor")
    } else if matches!(name.as_str(), "gemini" | "gemini.exe") {
        Some("gemini")
    } else if matches!(name.as_str(), "opencode" | "opencode.exe") {
        Some("opencode")
    } else if matches!(name.as_str(), "aider" | "aider.exe") {
        Some("aider")
    } else {
        None
    }
}
pub fn process_purpose(
    name: &str,
    exe: &str,
    args: &[String],
    agent: &str,
) -> (&'static str, &'static str) {
    let name = name.to_lowercase();
    let exe = exe.to_lowercase();
    let tokens: Vec<_> = args.iter().map(|s| s.to_lowercase()).collect();
    let has = |word: &str| {
        tokens.iter().any(|a| {
            a == word
                || Path::new(a).file_stem().is_some_and(|s| s == word)
                || a.contains(&format!("/node_modules/{word}/"))
                || a.contains(&format!("\\node_modules\\{word}\\"))
        })
    };
    if name.contains("renderer") || tokens.iter().any(|a| a == "--type=renderer") {
        return ("renderer", "Renderer process name or renderer flag");
    }
    if name.contains("gpu") || tokens.iter().any(|a| a == "--type=gpu-process") {
        return ("gpu", "GPU process name or GPU flag");
    }
    if name.contains("plugin")
        || has("extensionhost")
        || tokens.iter().any(|a| a.contains("extension-host"))
    {
        return ("extensions", "Extension host name or startup flag");
    }
    if tokens.iter().any(|a| a.contains("networkservice")) {
        return ("network", "Network service startup flag");
    }
    if has("mcp")
        || tokens.iter().any(|a| {
            a.contains("/modelcontextprotocol/")
                || a.contains("\\modelcontextprotocol\\")
                || a.starts_with("@modelcontextprotocol/")
                || a.starts_with("--mcp")
        })
    {
        return ("mcp", "MCP server executable or startup argument");
    }
    if ["vite", "next", "nuxt", "webpack-dev-server"]
        .iter()
        .any(|n| has(n))
    {
        return ("devserver", "Known development server entry point");
    }
    if [
        "esbuild", "tsc", "webpack", "rollup", "cargo", "rustc", "swiftc",
    ]
    .iter()
    .any(|n| has(n) || name == *n)
    {
        return ("build", "Known compiler or build tool entry point");
    }
    if ["vitest", "jest", "pytest", "playwright"]
        .iter()
        .any(|n| has(n))
    {
        return ("test", "Known test runner entry point");
    }
    if exe.contains("chrome.app/") || name.contains("chromium") || name.contains("webkit") {
        return (
            "browser",
            "Browser executable in an observed agent process tree",
        );
    }
    if identify(&name, &exe) == Some(agent) && !name.contains("helper") {
        return if agent == "cursor" {
            ("editor", "Editor application executable")
        } else {
            ("agent", "Agent command-line executable")
        };
    }
    if matches!(
        name.as_str(),
        "sh" | "zsh" | "bash" | "fish" | "cmd.exe" | "powershell.exe" | "pwsh.exe"
    ) {
        return (
            "shell",
            "Shell executable in an observed agent process tree",
        );
    }
    (
        "unknown",
        "Executable and ancestry identify ownership, but do not establish the task",
    )
}
pub fn processes(
    system: &mut System,
    previous: &[ProcessEntry],
    definitions: &[AgentDefinition],
    bindings: &[AgentBinding],
) -> Vec<ProcessEntry> {
    // The default refresh omits UID, argv and cwd on Linux. New descendants
    // need these fields on every sample, including after an exec or cwd change.
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .with_memory()
            .with_cpu()
            .with_disk_usage()
            .with_user(UpdateKind::Always)
            .with_exe(UpdateKind::Always)
            .with_cmd(UpdateKind::Always)
            .with_cwd(UpdateKind::Always)
            .with_tasks(),
    );
    system.refresh_memory();
    let own_user = sysinfo::get_current_pid()
        .ok()
        .and_then(|p| system.process(p))
        .and_then(|p| p.user_id())
        .cloned();
    if own_user.is_none() {
        return vec![];
    }
    // Monitoring helpers and the monitor's own webviews are not agent work.
    let mut excluded: HashSet<Pid> = sysinfo::get_current_pid().into_iter().collect();
    for _ in 0..64 {
        let children: Vec<_> = system
            .processes()
            .iter()
            .filter(|(id, p)| {
                !excluded.contains(id) && p.parent().is_some_and(|id| excluded.contains(&id))
            })
            .map(|(id, _)| *id)
            .collect();
        if children.is_empty() {
            break;
        }
        excluded.extend(children);
    }
    let mut matches: HashMap<Pid, (String, String)> = HashMap::new();
    for (pid, p) in system.processes() {
        if p.user_id() != own_user.as_ref() || excluded.contains(pid) {
            continue;
        }
        if let Some(a) = identify(
            &p.name().to_string_lossy(),
            &p.exe()
                .map(|e| e.to_string_lossy().to_string())
                .unwrap_or_default(),
        ) {
            matches.insert(
                *pid,
                (a.into(), "Executable name / application path".into()),
            );
        } else {
            let exe = p
                .exe()
                .map(|e| e.to_string_lossy().into_owned())
                .unwrap_or_default();
            let args = p
                .cmd()
                .iter()
                .map(|a| a.to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            let cwd = p
                .cwd()
                .map(|c| c.to_string_lossy().into_owned())
                .unwrap_or_default();
            let entry = adoption::entrypoint(&exe, &args, &cwd);
            if let Some(b) = bindings.iter().find(|b| {
                adoption::binding_matches(b, pid.as_u32(), p.start_time(), &exe)
                    && definitions.iter().any(|d| d.id == b.agent && d.enabled)
            }) {
                matches.insert(
                    *pid,
                    (b.agent.clone(), "User-adopted process identity".into()),
                );
            } else if let Some(d) = definitions.iter().find(|d| {
                d.enabled
                    && d.rule
                        .as_ref()
                        .is_some_and(|r| adoption::rule_matches(r, &exe, &entry))
            }) {
                matches.insert(
                    *pid,
                    (d.id.clone(), "Exact executable and entry point rule".into()),
                );
            }
        }
    }
    let inherit = |matches: &mut HashMap<Pid, (String, String)>| {
        for _ in 0..64 {
            let mut added = vec![];
            for (pid, p) in system.processes() {
                if matches.contains_key(pid)
                    || p.user_id() != own_user.as_ref()
                    || excluded.contains(pid)
                {
                    continue;
                }
                if let Some(parent) = p.parent().and_then(|id| system.process(id)) {
                    if parent.start_time() > p.start_time() {
                        continue;
                    }
                }
                if let Some((a, _)) = p.parent().and_then(|id| matches.get(&id)) {
                    added.push((*pid, (a.clone(), "Observed parent process chain".into())));
                }
            }
            if added.is_empty() {
                break;
            }
            matches.extend(added);
        }
    };
    inherit(&mut matches);
    let mut orphan_ids = HashSet::new();
    for old in previous {
        if definitions.iter().any(|d| d.id == old.agent && d.enabled)
            && old.evidence.contains("parent")
            && !matches.contains_key(&Pid::from_u32(old.pid))
            && !excluded.contains(&Pid::from_u32(old.pid))
        {
            if let Some(p) = system.process(Pid::from_u32(old.pid)) {
                if p.start_time() == old.start_time && p.user_id() == own_user.as_ref() {
                    matches.insert(
                        Pid::from_u32(old.pid),
                        (
                            old.agent.clone(),
                            "Previously observed parent chain; parent exited".into(),
                        ),
                    );
                    orphan_ids.insert(old.pid);
                }
            }
        }
    }
    inherit(&mut matches);
    let mut out = vec![];
    for (pid, (agent, evidence)) in matches {
        if let Some(p) = system.process(pid) {
            let executable = p
                .exe()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let name = p.name().to_string_lossy().to_string();
            let args: Vec<_> = p
                .cmd()
                .iter()
                .map(|a| a.to_string_lossy().into_owned())
                .collect();
            let (purpose, purpose_evidence) = if evidence == "User-adopted process identity"
                || evidence == "Exact executable and entry point rule"
            {
                ("agent", "Agent ownership explicitly configured by the user; task usefulness still requires review")
            } else {
                process_purpose(&name, &executable, &args, &agent)
            };
            let prior = previous
                .iter()
                .find(|old| old.pid == pid.as_u32() && old.start_time == p.start_time());
            let high_cpu_samples = if p.cpu_usage() >= 80.0 {
                prior.map_or(1, |old| old.high_cpu_samples.saturating_add(1))
            } else {
                0
            };
            let orphan = orphan_ids.contains(&pid.as_u32());
            let disk = crate::process_analysis::disk_counters(pid.as_u32());
            let (recommendation, reason) = if orphan {
                (
                    "review",
                    "Observed parent exited; confirm whether this background task is still needed",
                )
            } else if high_cpu_samples >= 3 {
                (
                    "review",
                    "CPU stayed above 80% for at least three observations; check task progress",
                )
            } else if p.memory() >= 1_500_000_000 {
                (
                    "review",
                    "Memory exceeds 1.5 GB; check the task before deciding to stop it",
                )
            } else if purpose == "unknown" {
                (
                    "inspect",
                    "Task purpose is uncertain; inspect its project and parent before stopping",
                )
            } else {
                ("keep", "Recognized working component; keep it while the related task or application is in use")
            };
            out.push(ProcessEntry {
                pid: pid.as_u32(),
                parent_pid: p.parent().map(|p| p.as_u32()),
                start_time: p.start_time(),
                name,
                agent,
                memory: p.memory(),
                cpu: p.cpu_usage(),
                status: format!("{:?}", p.status()),
                evidence,
                orphan,
                executable,
                purpose: purpose.into(),
                purpose_evidence: purpose_evidence.into(),
                recommendation: recommendation.into(),
                recommendation_reason: reason.into(),
                working_directory: p
                    .cwd()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                project_path: String::new(),
                project_evidence: String::new(),
                runtime_seconds: p.run_time(),
                memory_delta: prior.map_or(0, |old| p.memory() as i64 - old.memory as i64),
                cpu_delta: prior.map_or(0.0, |old| p.cpu_usage() - old.cpu),
                high_cpu_samples,
                observation: ProcessObservation {
                    disk_read: disk.map_or(p.disk_usage().total_read_bytes, |v| v.0),
                    disk_written: disk.map_or(p.disk_usage().total_written_bytes, |v| v.1),
                    io_readable: disk.is_some(),
                    watch_mode: args
                        .iter()
                        .any(|a| a == "--watch" || a == "watch" || a.starts_with("--watch=")),
                    ..Default::default()
                },
                assessment: ProcessAssessment::default(),
            });
            crate::process_analysis::observe(out.last_mut().unwrap(), prior, now());
        }
    }
    out.sort_by(|a, b| b.memory.cmp(&a.memory));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sessions_and_credentials_are_never_candidates() {
        let r = Root {
            id: "r".into(),
            path: "/fixture".into(),
            agent: "codex".into(),
            label: "Fixture".into(),
            custom: false,
            source: "manual".into(),
        };
        assert_eq!(
            classify(Path::new("sessions/a.jsonl"), &r),
            ("sessions", None)
        );
        assert_eq!(classify(Path::new("tmp/auth.json"), &r), ("settings", None));
        assert_eq!(classify(Path::new("tmp/a.db"), &r), ("settings", None));
        assert_eq!(
            classify(Path::new("tmp/a.txt"), &r),
            ("temporary", Some(86400))
        );
    }
    #[test]
    fn app_payloads_are_protected_and_structured_logs_keep_their_safety_checks() {
        let mut r = Root {
            id: "r".into(),
            path: "/fixture/cache".into(),
            agent: "codex".into(),
            label: "Cache".into(),
            custom: false,
            source: "cache".into(),
        };
        let policy = CleanupPolicy::default();
        for path in [
            "org.sparkle-project.Sparkle/Update/file.bin",
            "download/Tool.app/Contents/Resources/file.bin",
            "A.framework/Versions/A/file.bin",
            "Helper.bundle/file.bin",
        ] {
            assert_eq!(classify(Path::new(path), &r), ("application", None));
            assert_eq!(cleanup_age(Path::new(path), &r, &policy), None);
        }
        r.path = "/fixture/Tool.app/Contents/Resources".into();
        assert_eq!(cleanup_age(Path::new("payload.bin"), &r, &policy), None);
        r.agent = "lumo".into();
        r.custom = true;
        r.source = "adopted-logs".into();
        r.path = "/fixture/logs".into();
        assert_eq!(
            classify(Path::new("events.2026-11-01.jsonl"), &r),
            ("logs", Some(7 * 86400))
        );
        assert_eq!(
            cleanup_age(Path::new("events.2026-11-01.jsonl"), &r, &policy),
            Some(policy.log_days * 86400)
        );
        for path in [
            "auth.jsonl",
            "credentials.txt",
            "config.json",
            "state.sqlite",
            "source.ts",
            "sessions/history.jsonl",
        ] {
            assert_eq!(cleanup_age(Path::new(path), &r, &policy), None, "{path}");
        }
    }
    #[test]
    fn cleanup_policy_can_include_old_sessions_without_unprotecting_projects() {
        let global = Root {
            id: "r".into(),
            path: "/fixture/.codex".into(),
            agent: "codex".into(),
            label: "Codex".into(),
            custom: false,
            source: "global".into(),
        };
        let project = Root {
            id: "p".into(),
            path: "/fixture/project".into(),
            agent: "codex".into(),
            label: "Project".into(),
            custom: true,
            source: "session".into(),
        };
        let policy = CleanupPolicy {
            include_sessions: true,
            session_days: 7,
            ..Default::default()
        };
        assert_eq!(
            cleanup_age(Path::new("sessions/old.jsonl"), &global, &policy),
            Some(7 * 86400)
        );
        assert_eq!(
            cleanup_age(Path::new("tmp/old.bin"), &project, &policy),
            None
        );
    }
    #[test]
    fn extended_rules_protect_newest_sessions_and_apply_scope_exclusions() {
        let directory = tempfile::tempdir().unwrap();
        let base = directory.path();
        for (name, days) in [
            ("sessions/latest.jsonl", 10),
            ("sessions/older.jsonl", 20),
            ("archived_sessions/oldest.jsonl", 30),
            ("shell_snapshots/helper.sh", 30),
            ("shell_snapshots/config.json", 30),
            ("tmp/file.bin", 30),
        ] {
            let path = base.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "fixture").unwrap();
            fs::OpenOptions::new()
                .write(true)
                .open(&path)
                .unwrap()
                .set_times(fs::FileTimes::new().set_modified(
                    UNIX_EPOCH + std::time::Duration::from_secs(now() - days * 86400),
                ))
                .unwrap();
        }
        let root = Root {
            id: "global".into(),
            path: base.to_string_lossy().into_owned(),
            label: "Codex".into(),
            agent: "codex".into(),
            custom: false,
            source: "global".into(),
        };
        let mut policy = CleanupPolicy {
            include_sessions: true,
            session_days: 7,
            session_cache_enabled: true,
            keep_latest_sessions: 1,
            ..Default::default()
        };
        let mut stable = HashMap::new();
        let first = scan(
            std::slice::from_ref(&root),
            &HashMap::new(),
            &HashSet::new(),
            &mut stable,
            &policy,
        );
        for value in stable.values_mut() {
            value.1 = now() - 20;
        }
        let mut result = scan(
            std::slice::from_ref(&root),
            &HashMap::new(),
            &HashSet::new(),
            &mut stable,
            &policy,
        );
        apply_session_retention(&mut result.files, &policy);
        let get = |name: &str| {
            result
                .files
                .iter()
                .find(|f| Path::new(&f.relative_path) == Path::new(name))
                .unwrap()
        };
        assert!(!get("sessions/latest.jsonl").eligible);
        assert!(get("sessions/older.jsonl").eligible);
        assert!(get("archived_sessions/oldest.jsonl").eligible);
        assert!(get("shell_snapshots/helper.sh").eligible);
        assert!(!get("shell_snapshots/config.json").eligible);
        policy.session_scope = "archived".into();
        assert_eq!(
            cleanup_age(Path::new("sessions/older.jsonl"), &root, &policy),
            None
        );
        assert!(cleanup_age(Path::new("archived_sessions/oldest.jsonl"), &root, &policy).is_some());
        policy.temporary_enabled = false;
        assert_eq!(cleanup_age(Path::new("tmp/file.bin"), &root, &policy), None);
        policy.minimum_size_mb = 1;
        assert!(cleanup_exclusion(get("shell_snapshots/helper.sh"), &policy)
            .unwrap()
            .contains("minimum"));
        policy.minimum_size_mb = 0;
        policy.excluded_paths = vec![base.join("shell_snapshots").to_string_lossy().into_owned()];
        assert!(cleanup_exclusion(get("shell_snapshots/helper.sh"), &policy)
            .unwrap()
            .contains("path"));
        assert!(cleanup_exclusion(get("sessions/older.jsonl"), &policy).is_none());
        policy.excluded_agents = vec!["codex".into()];
        assert!(cleanup_exclusion(get("sessions/older.jsonl"), &policy)
            .unwrap()
            .contains("agent"));
        let mut project = root.clone();
        project.custom = true;
        policy.session_scope = "all".into();
        assert_eq!(
            cleanup_age(
                Path::new(".claude/sessions/history.jsonl"),
                &project,
                &policy
            ),
            None
        );
        policy.include_project_sessions = true;
        assert!(cleanup_age(
            Path::new(".claude/sessions/history.jsonl"),
            &project,
            &policy
        )
        .is_some());
        assert_eq!(
            cleanup_age(Path::new("src/file.jsonl"), &project, &policy),
            None
        );
        let old: CleanupPolicy =
            serde_json::from_str(r#"{"preset":"conservative","includeSessions":false}"#).unwrap();
        assert!(old.cache_enabled && old.temporary_enabled && old.logs_enabled);
        assert_eq!(old.keep_latest_sessions, 10);
        assert_eq!(first.files.len(), 6);
    }
    #[test]
    fn process_purpose_explains_common_children() {
        assert_eq!(
            process_purpose("esbuild", "/usr/local/bin/esbuild", &[], "codex").0,
            "build"
        );
        assert_eq!(
            process_purpose(
                "Cursor Helper (Renderer)",
                "/Applications/Cursor.app/Contents/MacOS/Cursor",
                &[],
                "cursor"
            )
            .0,
            "renderer"
        );
        assert_eq!(
            process_purpose(
                "node",
                "/usr/local/bin/node",
                &["/project/node_modules/vite/bin/vite.js".into()],
                "codex"
            )
            .0,
            "devserver"
        );
    }
    #[test]
    fn projects_are_discovered_from_session_metadata_and_respect_ignored_roots() {
        let home = tempfile::tempdir().unwrap();
        let home_path = fs::canonicalize(home.path()).unwrap();
        let project = home_path.join("agentlens");
        fs::create_dir_all(&project).unwrap();
        let session = Session {
            id: "session-1".into(),
            agent: "codex".into(),
            project: project.to_string_lossy().into_owned(),
            path: home_path
                .join("session.jsonl")
                .to_string_lossy()
                .into_owned(),
            size: 128,
            modified: now(),
            confidence: "metadata".into(),
        };

        let found = discover_projects(&home_path, &[], &[session.clone()], &[], &HashSet::new());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, project.to_string_lossy());
        assert_eq!(found[0].source, "session");
        assert!(found[0].custom);

        let ignored = HashSet::from([project.to_string_lossy().into_owned()]);
        assert!(discover_projects(&home_path, &[], &[session], &[], &ignored).is_empty());
    }
    #[test]
    fn bounded_scan_requires_stability_and_preserves_sessions() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("tmp")).unwrap();
        fs::create_dir(dir.path().join("sessions")).unwrap();
        let temp = dir.path().join("tmp/old.txt");
        fs::write(&temp, "garbage").unwrap();
        fs::File::options()
            .write(true)
            .open(&temp)
            .unwrap()
            .set_times(
                fs::FileTimes::new()
                    .set_modified(UNIX_EPOCH + std::time::Duration::from_secs(now() - 172800)),
            )
            .unwrap();
        fs::write(
            dir.path().join("sessions/s.jsonl"),
            "{\"type\":\"session_meta\",\"payload\":{\"id\":\"session-1\",\"cwd\":\"/project\"}}\n",
        )
        .unwrap();
        let r = Root {
            id: "r".into(),
            path: dir.path().to_string_lossy().into(),
            agent: "codex".into(),
            label: "Fixture".into(),
            custom: false,
            source: "manual".into(),
        };
        let mut stable = HashMap::new();
        let first = scan(
            &[r.clone()],
            &HashMap::new(),
            &HashSet::new(),
            &mut stable,
            &CleanupPolicy::default(),
        );
        assert!(!first.files.iter().any(|f| f.eligible));
        assert_eq!(first.sessions[0].id, "session-1");
        for (_, v) in stable.iter_mut() {
            v.1 -= 20;
        }
        let _second = scan(
            &[r.clone()],
            &HashMap::new(),
            &HashSet::new(),
            &mut stable,
            &CleanupPolicy::default(),
        );
        #[cfg(unix)]
        assert_eq!(_second.files.iter().filter(|f| f.eligible).count(), 1);
        let active = HashSet::from(["codex".to_string()]);
        assert!(!scan(
            &[r],
            &HashMap::new(),
            &active,
            &mut stable,
            &CleanupPolicy::default()
        )
        .files
        .iter()
        .any(|f| f.eligible));
    }
    #[cfg(unix)]
    #[test]
    fn symlink_escape_and_hardlinks_are_rejected() {
        use std::os::unix::fs::symlink;
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        fs::write(b.path().join("secret"), "keep").unwrap();
        symlink(b.path(), a.path().join("escape")).unwrap();
        assert!(!path_is_safe(&a.path().join("escape/secret"), a.path()));
        let file = a.path().join("file");
        fs::write(&file, "keep").unwrap();
        fs::hard_link(&file, a.path().join("alias")).unwrap();
        assert!(!links_safe(&file, &fs::metadata(&file).unwrap()));
    }
}

#[cfg(windows)]
pub fn windows_identity(path: &Path) -> Option<(String, u32)> {
    use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};
    #[repr(C)]
    #[derive(Default)]
    struct FileInformation {
        attributes: u32,
        created: [u32; 2],
        accessed: [u32; 2],
        written: [u32; 2],
        volume: u32,
        size_high: u32,
        size_low: u32,
        links: u32,
        index_high: u32,
        index_low: u32,
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetFileInformationByHandle(
            file: *mut std::ffi::c_void,
            info: *mut FileInformation,
        ) -> i32;
    }
    // FILE_READ_ATTRIBUTES + OPEN_REPARSE_POINT; never traverse a symlink at the leaf.
    let handle = fs::OpenOptions::new()
        .access_mode(0x80)
        .share_mode(7)
        .custom_flags(0x00200000 | 0x02000000)
        .open(path)
        .ok()?;
    let mut info = FileInformation::default();
    if unsafe { GetFileInformationByHandle(handle.as_raw_handle(), &mut info) } == 0
        || info.attributes & 0x400 != 0
    {
        return None;
    }
    Some((
        format!("{}:{}:{}", info.volume, info.index_high, info.index_low),
        info.links,
    ))
}

/// Current user's processes only; argv is never sent to the frontend.
pub fn process_candidates(system: &System, managed: &[ProcessEntry]) -> Vec<ProcessCandidate> {
    let Some(own_pid) = sysinfo::get_current_pid().ok() else {
        return vec![];
    };
    let Some(user) = system.process(own_pid).and_then(|p| p.user_id()) else {
        return vec![];
    };
    let mut excluded = HashSet::from([own_pid]);
    for _ in 0..64 {
        let children = system
            .processes()
            .iter()
            .filter(|(pid, p)| {
                !excluded.contains(pid) && p.parent().is_some_and(|id| excluded.contains(&id))
            })
            .map(|(pid, _)| *pid)
            .collect::<Vec<_>>();
        if children.is_empty() {
            break;
        }
        excluded.extend(children);
    }
    let mut ancestor = system.process(own_pid).and_then(|p| p.parent());
    for _ in 0..64 {
        let Some(pid) = ancestor else {
            break;
        };
        excluded.insert(pid);
        ancestor = system.process(pid).and_then(|p| p.parent());
    }
    let mut out = system
        .processes()
        .iter()
        .filter(|(pid, p)| {
            p.user_id() == Some(user)
                && !excluded.contains(pid)
                && p.exe().is_some_and(|e| e.is_absolute())
        })
        .map(|(pid, p)| {
            let exe = p.exe().unwrap().to_string_lossy().into_owned();
            let cwd = p
                .cwd()
                .map(|c| c.to_string_lossy().into_owned())
                .unwrap_or_default();
            let args = p
                .cmd()
                .iter()
                .map(|a| a.to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            let entry = adoption::entrypoint(&exe, &args, &cwd);
            ProcessCandidate {
                pid: pid.as_u32(),
                parent_pid: p.parent().map(|p| p.as_u32()),
                start_time: p.start_time(),
                name: p.name().to_string_lossy().into_owned(),
                can_remember: adoption::validate_rule(&adoption::ProcessRule {
                    executable: exe.clone(),
                    entrypoint: entry.clone(),
                })
                .is_ok(),
                executable: exe,
                working_directory: cwd,
                entrypoint: entry,
                memory: p.memory(),
                cpu: p.cpu_usage(),
                agent: managed
                    .iter()
                    .find(|m| m.pid == pid.as_u32() && m.start_time == p.start_time())
                    .map(|m| m.agent.clone()),
            }
        })
        .collect::<Vec<_>>();
    out.sort_by(|a, b| b.memory.cmp(&a.memory).then(a.pid.cmp(&b.pid)));
    out
}

#[cfg(test)]
mod adopted_cleanup_tests {
    use super::*;
    #[test]
    fn explicit_custom_cache_uses_policy_but_never_unprotects_source_credentials_or_histories() {
        let mut root = Root {
            id: "custom".into(),
            path: "/home/test/cache".into(),
            agent: "custom-agent".into(),
            label: "Cache".into(),
            custom: true,
            source: "adopted-cache".into(),
        };
        let policy = CleanupPolicy {
            cache_days: 3,
            include_sessions: true,
            include_project_sessions: true,
            ..Default::default()
        };
        assert_eq!(
            cleanup_age(Path::new("old.bin"), &root, &policy),
            Some(3 * 86400)
        );
        for path in [
            "auth.json",
            "a.sqlite",
            "index.ts",
            "secret.txt",
            "sessions/old.jsonl",
            "history.jsonl",
            "config.yaml",
            ".env.local",
        ] {
            assert_eq!(cleanup_age(Path::new(path), &root, &policy), None, "{path}");
        }
        root.source = "adopted-project".into();
        assert_eq!(
            cleanup_age(Path::new("cache/old.bin"), &root, &policy),
            None
        );
        root.source = "adopted-logs".into();
        assert_eq!(
            cleanup_age(Path::new("old.log"), &root, &policy),
            Some(policy.log_days * 86400)
        );
        root.source = "adopted-temporary".into();
        assert_eq!(
            cleanup_age(Path::new("old.bin"), &root, &policy),
            Some(policy.temporary_hours * 3600)
        );
    }
}
