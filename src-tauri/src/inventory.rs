use crate::{collector::*, model::*};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    path::Path,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Condvar, Mutex,
    },
    time::Instant,
};

pub fn worker_count() -> usize {
    std::thread::available_parallelism()
        .map_or(2, usize::from)
        .clamp(2, 6)
}

/// Parallelism is bounded; callbacks are throttled so IPC never runs once per file.
pub struct Progress<'a> {
    state: Mutex<ScanProgress>,
    last: Mutex<Instant>,
    started: Instant,
    emit: &'a (dyn Fn(ScanProgress) + Sync),
    cancelled: &'a (dyn Fn() -> bool + Sync),
}
impl<'a> Progress<'a> {
    pub fn new(
        initial: bool,
        emit: &'a (dyn Fn(ScanProgress) + Sync),
        cancelled: &'a (dyn Fn() -> bool + Sync),
    ) -> Self {
        Self {
            state: Mutex::new(ScanProgress {
                initial,
                workers: worker_count(),
                phase: "discovery".into(),
                ..Default::default()
            }),
            last: Mutex::new(Instant::now()),
            started: Instant::now(),
            emit,
            cancelled,
        }
    }
    pub fn update(&self, force: bool, update: impl FnOnce(&mut ScanProgress)) {
        let mut state = self.state.lock().unwrap();
        update(&mut state);
        state.elapsed_ms = self.started.elapsed().as_millis() as u64;
        let mut last = self.last.lock().unwrap();
        if force || last.elapsed().as_millis() >= 250 {
            *last = Instant::now();
            (self.emit)(state.clone());
        }
    }
    pub fn cancelled(&self) -> bool {
        (self.cancelled)()
    }
}

fn excluded(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        ".trash"
            | ".trashes"
            | "$recycle.bin"
            | ".cache"
            | ".local"
            | ".config"
            | ".bun"
            | ".yarn"
            | ".npm"
            | ".pnpm-store"
            | ".cargo"
            | ".rustup"
            | "node_modules"
            | ".git"
            | "target"
            | "dist"
            | "build"
            | ".next"
            | ".nuxt"
            | ".venv"
            | "venv"
            | "__pycache__"
            | ".tox"
            | ".gradle"
            | "vendor"
            | ".codex"
            | ".claude"
            | ".cursor"
            | ".gemini"
            | ".opencode"
            | ".aider"
    ) || name.ends_with(".app")
        || name.ends_with(".photoslibrary")
}
fn marker(name: &str) -> Option<&'static str> {
    match name {
        ".codex" => Some("codex"),
        "AGENTS.md" => Some("unknown"),
        ".claude" | "CLAUDE.md" => Some("claude"),
        ".cursor" | ".cursorrules" => Some("cursor"),
        ".gemini" | "GEMINI.md" => Some("gemini"),
        ".opencode" | "opencode.json" | "opencode.jsonc" => Some("opencode"),
        ".aider" | ".aider.chat.history.md" | ".aider.conf.yml" => Some("aider"),
        ".git" | "package.json" | "Cargo.toml" | "pyproject.toml" | "go.mod" | "pom.xml" => {
            Some("unknown")
        }
        _ => None,
    }
}

/// Walk all accessible user directories, without following links or entering OS,
/// dependency and generated directory trees. No source or conversation content is read.
pub fn discover_home(
    home: &Path,
    roots: &[Root],
    ignored: &HashSet<String>,
    progress: &Progress<'_>,
) -> (Vec<Root>, Vec<String>) {
    let home = fs::canonicalize(home).unwrap_or_else(|_| home.to_path_buf());
    let queue = (
        Mutex::new((VecDeque::from([home.clone()]), 0usize)),
        Condvar::new(),
    );
    let found = Mutex::new(Vec::new());
    let warnings = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..worker_count() {
            let queue = &queue;
            let home = &home;
            let found = &found;
            let warnings = &warnings;
            scope.spawn(move || loop {
                let directory = {
                    let (lock, wake) = queue;
                    let mut state = lock.lock().unwrap();
                    loop {
                        if progress.cancelled() {
                            return;
                        }
                        if let Some(path) = state.0.pop_front() {
                            state.1 += 1;
                            break path;
                        }
                        if state.1 == 0 {
                            return;
                        }
                        state = wake.wait(state).unwrap();
                    }
                };
                let mut children = Vec::new();
                let mut agent = None;
                // Home children can trigger a blocking OS folder permission prompt.
                // Publish the attempted path first, without emitting per-directory IPC
                // for the much larger project subtree traversal.
                progress.update(
                    directory == *home || directory.parent() == Some(home),
                    |p| {
                        p.current_path = directory.to_string_lossy().into_owned();
                    },
                );
                match fs::read_dir(&directory) {
                    Ok(entries) => {
                        for entry in entries {
                            let Ok(entry) = entry else {
                                progress.update(false, |p| p.errors += 1);
                                continue;
                            };
                            let Ok(kind) = entry.file_type() else {
                                progress.update(false, |p| p.errors += 1);
                                continue;
                            };
                            if kind.is_symlink() {
                                continue;
                            }
                            let name = entry.file_name().to_string_lossy().into_owned();
                            if let Some(a) = marker(&name) {
                                if agent.is_none() || agent == Some("unknown") {
                                    agent = Some(a);
                                }
                            }
                            let path = entry.path();
                            if kind.is_dir()
                                && !(directory == *home
                                    && matches!(name.as_str(), "Library" | "AppData"))
                                && !excluded(&name)
                                && !ignored.iter().any(|r| path.starts_with(r))
                                && !roots.iter().any(|r| path.starts_with(&r.path))
                            {
                                children.push(path);
                            }
                        }
                    }
                    Err(error) => {
                        progress.update(false, |p| p.errors += 1);
                        let mut notes = warnings.lock().unwrap();
                        if notes.len() < 20 {
                            notes.push(format!("{}: {}", directory.display(), error));
                        }
                    }
                }
                if directory != *home && !ignored.iter().any(|r| directory.starts_with(r)) {
                    if let Some(agent) = agent {
                        let mut items = found.lock().unwrap();
                        let path = directory.to_string_lossy().into_owned();
                        items.push(Root {
                            id: path.clone(),
                            path,
                            label: directory
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned(),
                            agent: agent.into(),
                            custom: true,
                            source: "scan".into(),
                        });
                        progress.update(false, |p| p.projects = items.len());
                    }
                }
                progress.update(false, |p| {
                    p.directories += 1;
                    p.current_path = directory.to_string_lossy().into_owned();
                });
                let (lock, wake) = queue;
                let mut state = lock.lock().unwrap();
                state.0.extend(children);
                state.1 -= 1;
                wake.notify_all();
            });
        }
    });
    // Outer project roots include nested packages; count their files only once.
    let mut candidates = found.into_inner().unwrap();
    candidates.sort_by_key(|r| Path::new(&r.path).components().count());
    let mut selected: Vec<Root> = Vec::new();
    for candidate in candidates {
        if !roots.iter().chain(selected.iter()).any(|r| {
            Path::new(&candidate.path).starts_with(&r.path)
                || Path::new(&r.path).starts_with(&candidate.path)
        }) {
            selected.push(candidate);
        }
    }
    progress.update(true, |p| p.projects = selected.len());
    (selected, warnings.into_inner().unwrap())
}

pub fn scan_parallel(
    roots: &[Root],
    active: &HashSet<String>,
    stable: &mut HashMap<String, (String, u64)>,
    policy: &CleanupPolicy,
    progress: &Progress<'_>,
) -> Scan {
    let next = AtomicUsize::new(0);
    let results = Mutex::new(Vec::new());
    // Move stability observations into each root's worker without cloning the inventory.
    let mut observations: Vec<_> = roots.iter().map(|_| HashMap::new()).collect();
    for (path, observation) in stable.drain() {
        if let Some(index) = roots
            .iter()
            .position(|r| Path::new(&path).starts_with(&r.path))
        {
            observations[index].insert(path, observation);
        }
    }
    let observations = Mutex::new(observations);
    progress.update(true, |p| {
        p.phase = "indexing".into();
        p.roots_total += roots.len();
    });
    std::thread::scope(|scope| {
        for _ in 0..worker_count().min(roots.len()) {
            let next = &next;
            let results = &results;
            let observations = &observations;
            scope.spawn(move || loop {
                if progress.cancelled() {
                    break;
                }
                let index = next.fetch_add(1, Ordering::Relaxed);
                let Some(root) = roots.get(index) else {
                    break;
                };
                progress.update(true, |p| p.current_path = root.path.clone());
                let mut stable = std::mem::take(&mut observations.lock().unwrap()[index]);
                let result = scan_with_progress(
                    std::slice::from_ref(root),
                    &HashMap::new(),
                    active,
                    &mut stable,
                    policy,
                    &|bytes, path| {
                        progress.update(false, |p| {
                            p.files += 1;
                            p.bytes += bytes;
                            p.current_path = path.to_string_lossy().into_owned();
                        });
                        !progress.cancelled()
                    },
                );
                progress.update(true, |p| {
                    p.roots_done += 1;
                    p.errors += result.warnings.len() as u64;
                });
                results.lock().unwrap().push((index, result, stable));
            });
        }
    });
    let mut result = Scan {
        files: Vec::new(),
        sessions: Vec::new(),
        warnings: Vec::new(),
        complete: !progress.cancelled(),
    };
    let mut pieces = results.into_inner().unwrap();
    pieces.sort_by_key(|(index, _, _)| *index);
    let mut counted = HashSet::new();
    for (_, part, observations) in pieces {
        result.complete &= part.complete;
        result.files.extend(
            part.files
                .into_iter()
                .filter(|f| counted.insert(f.path.clone())),
        );
        result.sessions.extend(part.sessions);
        result.warnings.extend(part.warnings);
        stable.extend(observations);
    }
    result
        .files
        .sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.path.cmp(&b.path)));
    result.sessions.sort_by(|a, b| b.modified.cmp(&a.modified));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn home_discovery_finds_old_nested_projects_and_skips_dependencies_and_ignored_paths() {
        let directory = tempfile::tempdir().unwrap();
        let home = fs::canonicalize(directory.path()).unwrap();
        for (project, marker) in [
            ("Code/old-project", ".claude"),
            ("single-project", ".git"),
            ("Code/monorepo", ".codex"),
            ("Code/monorepo/packages/child", ".git"),
            ("Code/node_modules/ignored", ".codex"),
            ("Library/ignored", ".claude"),
            ("removed", ".codex"),
        ] {
            fs::create_dir_all(home.join(project).join(marker)).unwrap();
        }
        #[cfg(unix)]
        {
            let outside = tempfile::tempdir().unwrap();
            fs::create_dir(outside.path().join(".claude")).unwrap();
            std::os::unix::fs::symlink(outside.path(), home.join("linked")).unwrap();
        }
        let updates = Mutex::new(Vec::new());
        let emit = |p: ScanProgress| updates.lock().unwrap().push(p);
        let progress = Progress::new(true, &emit, &|| false);
        let ignored = HashSet::from([home.join("removed").to_string_lossy().into_owned()]);
        let (roots, warnings) = discover_home(&home, &[], &ignored, &progress);
        assert!(warnings.is_empty());
        assert_eq!(roots.len(), 3);
        assert!(roots
            .iter()
            .any(|r| r.label == "old-project" && r.agent == "claude"));
        assert!(roots
            .iter()
            .any(|r| r.label == "single-project" && r.agent == "unknown"));
        assert!(roots
            .iter()
            .any(|r| r.label == "monorepo" && r.agent == "codex"));
        assert!(updates.lock().unwrap().last().unwrap().directories > 0);
    }
    #[test]
    fn parallel_inventory_is_complete_deduplicated_and_reports_progress() {
        let directory = tempfile::tempdir().unwrap();
        let mut roots = Vec::new();
        for i in 0..8 {
            let path = directory.path().join(format!("project-{i}"));
            fs::create_dir_all(path.join(".codex")).unwrap();
            fs::create_dir_all(path.join("node_modules/ignored")).unwrap();
            fs::write(path.join("node_modules/ignored/large.bin"), [0; 100]).unwrap();
            fs::write(
                path.join(".codex/session.jsonl"),
                format!(
                    "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"{i}\",\"cwd\":\"{}\"}}}}\n",
                    path.display()
                ),
            )
            .unwrap();
            fs::write(path.join("source.rs"), "source").unwrap();
            roots.push(Root {
                id: i.to_string(),
                label: i.to_string(),
                path: path.to_string_lossy().into_owned(),
                agent: "codex".into(),
                custom: true,
                source: "scan".into(),
            });
        }
        let updates = Mutex::new(Vec::new());
        let emit = |p: ScanProgress| updates.lock().unwrap().push(p);
        let progress = Progress::new(true, &emit, &|| false);
        let result = scan_parallel(
            &roots,
            &HashSet::new(),
            &mut HashMap::new(),
            &CleanupPolicy::default(),
            &progress,
        );
        assert!(result.complete);
        assert_eq!(result.files.len(), 16);
        assert_eq!(result.sessions.len(), 8);
        assert!(result.files.iter().all(|f| !f.eligible));
        let states = updates.lock().unwrap();
        let last = states.last().unwrap();
        assert_eq!(last.files, 16);
        assert_eq!(last.roots_done, 8);
        assert_eq!(last.bytes, result.files.iter().map(|f| f.size).sum::<u64>());
        assert!(states.windows(2).all(|p| p[0].files <= p[1].files));
    }
    #[test]
    fn cancelled_discovery_and_indexing_finish_without_reporting_completion() {
        let directory = tempfile::tempdir().unwrap();
        let progress = Progress::new(true, &|_| {}, &|| true);
        assert!(
            discover_home(directory.path(), &[], &HashSet::new(), &progress)
                .0
                .is_empty()
        );
        assert!(
            !scan_parallel(
                &[],
                &HashSet::new(),
                &mut HashMap::new(),
                &CleanupPolicy::default(),
                &progress
            )
            .complete
        );
    }
}
