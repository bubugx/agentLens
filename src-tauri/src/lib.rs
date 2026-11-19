mod activity;
mod adoption;
mod process_analysis;
mod resources;
use adoption::*;
mod collector;
mod inventory;
mod model;
use inventory::*;
mod store;
use collector::*;
use model::*;
use notify::Watcher;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Condvar, Mutex, RwLock,
    },
    time::Duration,
};
use store::Store;
use sysinfo::{Disks, Pid, Signal, System};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager,
};

struct Core {
    store: Store,
    roots: Vec<Root>,
    custom_agents: Vec<AgentDefinition>,
    bindings: Vec<AgentBinding>,
    process_activity_initialized: bool,
    resource_sample_at: u64,
    resource_keys: HashSet<(String, String)>,
    system: System,
    files: HashMap<String, FileEntry>,
    stable: HashMap<String, (String, u64)>,
    plans: HashMap<String, (CleanupPlan, HashMap<String, String>)>,
    baseline: HashMap<String, u64>,
    settings: Settings,
    ignored_roots: HashSet<String>,
    data_path: PathBuf,
}
impl Core {
    fn collect_processes(&mut self, previous: &[ProcessEntry]) -> Vec<ProcessEntry> {
        let mut current = processes(
            &mut self.system,
            previous,
            &definitions(&self.custom_agents),
            &self.bindings,
        );
        resources::assign_projects(&mut current, previous, &self.roots);
        let mut protected = HashSet::new();
        let mut ancestor = sysinfo::get_current_pid().ok();
        for _ in 0..64 {
            let Some(pid) = ancestor else { break };
            protected.insert(pid.as_u32());
            ancestor = self.system.process(pid).and_then(|p| p.parent());
        }
        process_analysis::assess(&mut current, &protected, now());
        current
    }
    fn adoption_preview(
        &mut self,
        request: &AdoptRequest,
        previous: &[ProcessEntry],
    ) -> Result<AdoptionPreview, String> {
        let name = request.name.trim();
        if name.is_empty()
            || name.chars().count() > 64
            || name.chars().any(char::is_control)
            || request.color.len() != 7
            || !request.color.starts_with('#')
            || !request.color[1..].bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("Enter a name of 1–64 characters and a valid color".into());
        }
        if definitions(&self.custom_agents)
            .iter()
            .any(|d| d.enabled && d.name.to_lowercase() == name.to_lowercase())
        {
            return Err("An Agent with this name already exists".into());
        }
        let current = self.collect_processes(previous);
        let candidates = process_candidates(&self.system, &current);
        let pinned = if let Some(pid) = request.pid {
            let candidate = candidates
                .iter()
                .find(|p| {
                    p.pid == pid
                        && Some(p.start_time) == request.start_time
                        && p.executable == request.executable
                })
                .ok_or("Process exited or its identity changed")?;
            if candidate.agent.is_some() {
                return Err("This process is already managed by an Agent".into());
            }
            Some(candidate)
        } else {
            None
        };
        if pinned.is_none() && !request.remember {
            return Err("Choose a running process or save a recognition rule".into());
        }
        let rule = if request.remember {
            let rule = ProcessRule {
                executable: request.executable.clone(),
                entrypoint: request.entrypoint.clone(),
            };
            validate_rule(&rule)?;
            if pinned.is_some_and(|p| !rule_matches(&rule, &p.executable, &p.entrypoint)) {
                return Err("The rule does not match the selected process entry point".into());
            }
            if self
                .custom_agents
                .iter()
                .any(|d| d.enabled && d.rule.as_ref() == Some(&rule))
            {
                return Err("This recognition rule is already in use".into());
            }
            // Built-in identities take precedence; reject incompatible rules up front.
            if collector::identify(
                Path::new(&rule.executable)
                    .file_name()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or(""),
                &rule.executable,
            )
            .is_some()
            {
                return Err("This executable is already recognized by a built-in Agent".into());
            }
            Some(rule)
        } else {
            None
        };
        let mut selected: HashSet<u32> = candidates
            .iter()
            .filter(|p| {
                pinned.is_some_and(|root| root.pid == p.pid)
                    || rule
                        .as_ref()
                        .is_some_and(|r| rule_matches(r, &p.executable, &p.entrypoint))
            })
            .map(|p| p.pid)
            .collect();
        for _ in 0..64 {
            let children = candidates
                .iter()
                .filter(|p| {
                    !selected.contains(&p.pid)
                        && p.parent_pid.is_some_and(|id| selected.contains(&id))
                        && p.parent_pid
                            .and_then(|id| candidates.iter().find(|p| p.pid == id))
                            .is_some_and(|parent| parent.start_time <= p.start_time)
                })
                .map(|p| p.pid)
                .collect::<Vec<_>>();
            if children.is_empty() {
                break;
            }
            selected.extend(children);
        }
        let matches = candidates
            .into_iter()
            .filter(|p| selected.contains(&p.pid))
            .collect::<Vec<_>>();
        if matches.iter().any(|p| p.agent.is_some()) {
            return Err(
                "This process tree or rule includes processes already managed by another Agent"
                    .into(),
            );
        }
        let roots = validate_directories(
            &request.directories,
            &self.roots,
            &dirs::home_dir().ok_or("Home unavailable")?,
            "preview",
        )?;
        Ok(AdoptionPreview {
            processes: matches,
            roots,
            rule,
        })
    }
}
#[derive(Clone)]
struct Engine {
    core: Arc<Mutex<Core>>,
    view: Arc<RwLock<Snapshot>>,
    dirty: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    forced: Arc<AtomicBool>,
    running: Arc<AtomicBool>,
    full_scan: Arc<AtomicBool>,
    revision: Arc<AtomicU64>,
    wake: Arc<(Mutex<bool>, Condvar)>,
}
fn blank(
    settings: Settings,
    roots: Vec<Root>,
    history: Vec<HistoryPoint>,
    activity: Vec<Activity>,
) -> Snapshot {
    Snapshot {
        memory_delta: 0,
        cpu_delta: 0.0,
        timestamp: now(),
        scanned_at: 0,
        monitoring: true,
        scanning: true,
        platform: std::env::consts::OS.into(),
        total_memory: 0,
        used_memory: 0,
        disk_total: 0,
        disk_available: 0,
        agents: vec![],
        agent_definitions: definitions(&[]),
        roots,
        files: vec![],
        file_count: 0,
        sessions: vec![],
        processes: vec![],
        history,
        activity,
        warnings: vec![],
        settings,
        scan_complete: false,
        cleanup_summary: vec![],
        scan_progress: ScanProgress::default(),
    }
}
impl Engine {
    fn new(path: &Path) -> Result<Self, String> {
        let store = Store::open(path)?;
        let initial = store.get::<u32>("full_discovery_version") != Some(1);
        let mut settings = store.get::<Settings>("settings").unwrap_or_default();
        if settings.theme == "glass" {
            settings.theme = "light".into();
            store.set("settings", &settings)?;
        }
        let ignored_roots = store
            .get::<HashSet<String>>("ignored_roots")
            .unwrap_or_default();
        let mut roots = discover();
        for r in store.get::<Vec<Root>>("roots").unwrap_or_default() {
            if !roots.iter().any(|a| {
                Path::new(&a.path).starts_with(&r.path) || Path::new(&r.path).starts_with(&a.path)
            }) {
                roots.push(r)
            }
        }
        let custom_agents = store
            .get::<Vec<AgentDefinition>>("agent_definitions")
            .unwrap_or_default();
        let history = store.history();
        let activity = store.activity();
        let baseline = store.get("baseline").unwrap_or_default();
        let mut snapshot = blank(settings.clone(), roots.clone(), history, activity);
        snapshot.agent_definitions = definitions(&custom_agents);
        snapshot.scan_progress.initial = initial;
        if store.interrupted() {
            snapshot.warnings.push("An interrupted cleanup needs review. Inspect Trash and original paths; it has not been retried.".into())
        }
        Ok(Self {
            core: Arc::new(Mutex::new(Core {
                store,
                roots,
                custom_agents,
                bindings: vec![],
                process_activity_initialized: false,
                resource_sample_at: 0,
                resource_keys: HashSet::new(),
                system: System::new_all(),
                files: HashMap::new(),
                stable: HashMap::new(),
                plans: HashMap::new(),
                baseline,
                settings,
                ignored_roots,
                data_path: path.to_path_buf(),
            })),
            view: Arc::new(RwLock::new(snapshot)),
            dirty: Arc::new(AtomicBool::new(true)),
            paused: Arc::new(AtomicBool::new(false)),
            forced: Arc::new(AtomicBool::new(false)),
            running: Arc::new(AtomicBool::new(false)),
            full_scan: Arc::new(AtomicBool::new(initial)),
            revision: Arc::new(AtomicU64::new(0)),
            wake: Arc::new((Mutex::new(false), Condvar::new())),
        })
    }
    fn request_scan(&self) {
        self.forced.store(true, Ordering::Relaxed);
        let (lock, signal) = &*self.wake;
        *lock.lock().unwrap() = true;
        signal.notify_one();
    }
    fn sample(&self) -> Result<Snapshot, String> {
        let mut core = self.core.lock().map_err(|_| "Collector lock failed")?;
        let mut view = self
            .view
            .read()
            .map_err(|_| "Snapshot unavailable")?
            .clone();
        view.monitoring = !self.paused.load(Ordering::Relaxed);
        view.timestamp = now();
        view.settings = core.settings.clone();
        view.agent_definitions = definitions(&core.custom_agents);
        view.roots = core.roots.clone();
        if !view.monitoring {
            view.scanning = self.running.load(Ordering::Relaxed);
            *self.view.write().unwrap() = view.clone();
            return Ok(view);
        }
        let current_processes = core.collect_processes(&view.processes);
        let before_active: HashSet<_> = view.processes.iter().map(|p| p.agent.as_str()).collect();
        let after_active: HashSet<_> = current_processes.iter().map(|p| p.agent.as_str()).collect();
        if core.process_activity_initialized && before_active != after_active {
            self.request_scan();
        }
        for event in activity::process_events(
            &view.processes,
            &current_processes,
            core.process_activity_initialized,
        ) {
            core.store.event(&event)?;
            view.activity.insert(0, event);
        }
        core.process_activity_initialized = true;
        view.processes = current_processes;
        if now().saturating_sub(core.resource_sample_at) >= 30 {
            let mut samples = resources::aggregate(&view.processes, now());
            for key in &core.resource_keys {
                samples
                    .entry(key.clone())
                    .or_insert(resources::ResourcePoint {
                        time: now(),
                        memory: 0,
                        cpu: 0.,
                        processes: 0,
                    });
            }
            core.store.resource_samples(&samples)?;
            core.resource_keys = resources::aggregate(&view.processes, now())
                .into_keys()
                .collect();
            core.resource_sample_at = now();
        }
        view.total_memory = core.system.total_memory();
        view.used_memory = core.system.used_memory();
        let prior_memory: u64 = view.agents.iter().map(|a| a.memory).sum();
        let prior_cpu: f32 = view.agents.iter().map(|a| a.cpu).sum();
        let had_prior = !view.agents.is_empty();
        view.agents = definitions(&core.custom_agents)
            .iter()
            .filter(|d| d.enabled)
            .map(|definition| {
                let id = &definition.id;
                let name = &definition.name;
                let files: Vec<_> = core.files.values().filter(|f| f.agent == *id).collect();
                let procs: Vec<_> = view.processes.iter().filter(|p| p.agent == *id).collect();
                let storage = files.iter().map(|f| f.size).sum();
                let file_count = files.len();
                let session_count = files.iter().filter(|f| f.session_id.is_some()).count();
                let reclaimable = files
                    .iter()
                    .filter(|f| f.eligible && procs.is_empty())
                    .map(|f| f.size)
                    .sum();
                drop(files);
                let baseline = if view.scanned_at > 0 {
                    *core.baseline.entry(id.to_string()).or_insert(storage)
                } else {
                    storage
                };
                AgentSummary {
                    id: id.to_string(),
                    name: name.to_string(),
                    storage,
                    memory: procs.iter().map(|p| p.memory).sum(),
                    cpu: procs.iter().map(|p| p.cpu).sum(),
                    files: file_count,
                    sessions: session_count,
                    processes: procs.len(),
                    reclaimable,
                    growth: storage as i64 - baseline as i64,
                    installed: view.roots.iter().any(|r| r.agent == *id) || !procs.is_empty(),
                }
            })
            .collect();
        view.memory_delta = if had_prior {
            view.agents.iter().map(|a| a.memory).sum::<u64>() as i64 - prior_memory as i64
        } else {
            0
        };
        view.cpu_delta = if had_prior {
            view.agents.iter().map(|a| a.cpu).sum::<f32>() - prior_cpu
        } else {
            0.0
        };
        if view.scanned_at > 0
            && !self.running.load(Ordering::Relaxed)
            && view.scan_complete
            && view
                .history
                .last()
                .is_none_or(|p| now().saturating_sub(p.time) >= 60)
        {
            let p = HistoryPoint {
                time: now(),
                storage: view.agents.iter().map(|a| a.storage).sum(),
                memory: view.agents.iter().map(|a| a.memory).sum(),
                cpu: view.agents.iter().map(|a| a.cpu).sum(),
                agents: view
                    .agents
                    .iter()
                    .map(|a| (a.id.clone(), a.storage))
                    .collect(),
            };
            core.store.point(&p, core.settings.retention_days)?;
            view.history = core.store.history();
        }
        for file in &mut view.files {
            if file.eligible && view.processes.iter().any(|p| p.agent == file.agent) {
                file.eligible = false;
                file.reason = "Agent is running; retained".into();
                if file.cleanup_status == "eligible" {
                    file.cleanup_status = "active".into();
                }
            }
        }
        activity::bound(&mut view.activity);
        let mut current = self.view.write().unwrap();
        view.scanning = current.scanning;
        view.scan_progress = current.scan_progress.clone();
        *current = view.clone();
        Ok(view)
    }
}
impl Engine {
    fn maybe_scan(&self, app: tauri::AppHandle) {
        if self.paused.load(Ordering::Relaxed) || self.running.load(Ordering::Relaxed) {
            return;
        }
        let scanned_at = self.view.read().unwrap().scanned_at;
        if !(self.forced.load(Ordering::Relaxed)
            || (self.dirty.load(Ordering::Relaxed) && now().saturating_sub(scanned_at) >= 30)
            || now().saturating_sub(scanned_at) >= 300)
        {
            return;
        }
        if self.running.swap(true, Ordering::Relaxed) {
            return;
        }
        let foreground = self.forced.swap(false, Ordering::Relaxed);
        self.dirty.store(false, Ordering::Relaxed);
        let engine = self.clone();
        std::thread::Builder::new()
            .name("agentlens-inventory".into())
            .spawn(move || {
                let _ = engine.sample();
                let result = engine.run_inventory(foreground, &|progress| {
                    {
                        let mut view = engine.view.write().unwrap();
                        view.scanning =
                            !matches!(progress.phase.as_str(), "complete" | "paused" | "error");
                        view.scan_progress = progress.clone();
                    }
                    let _ = app.emit("scan-progress", &progress);
                });
                if let Err(error) = result {
                    let mut view = engine.view.write().unwrap();
                    view.scanning = false;
                    view.scan_progress.phase = "error".into();
                    view.warnings
                        .insert(0, format!("Inventory issue: {}", error));
                    let _ = app.emit("scan-progress", &view.scan_progress);
                }
                engine.running.store(false, Ordering::Relaxed);
                if let Ok(snapshot) = engine.sample() {
                    let _ = app.emit("snapshot", &snapshot);
                }
            })
            .expect("Unable to start inventory worker");
    }
    fn run_inventory(
        &self,
        foreground: bool,
        emit: &(dyn Fn(ScanProgress) + Sync),
    ) -> Result<(), String> {
        let revision = self.revision.load(Ordering::Relaxed);
        let (mut roots, ignored, policy, auto_discover, mut stable, data_path, initial) = {
            let mut core = self.core.lock().unwrap();
            (
                core.roots.clone(),
                core.ignored_roots.clone(),
                core.settings.cleanup.clone(),
                core.settings.auto_discover,
                std::mem::take(&mut core.stable),
                core.data_path.clone(),
                core.store.get::<u32>("full_discovery_version") != Some(1),
            )
        };
        let full = self.full_scan.swap(false, Ordering::Relaxed) || initial;
        let cancelled = || {
            self.paused.load(Ordering::Relaxed) || self.revision.load(Ordering::Relaxed) != revision
        };
        let progress = Progress::new(initial, emit, &cancelled);
        progress.update(true, |p| {
            p.phase = if full { "discovery" } else { "indexing" }.into();
            p.background = !initial && !full && !foreground;
        });
        let before_roots: HashSet<_> = roots.iter().map(|r| r.path.clone()).collect();
        let mut discovery_warnings = Vec::new();
        if full && auto_discover {
            if let Some(home) = dirs::home_dir() {
                let (found, warnings) = discover_home(&home, &roots, &ignored, &progress);
                roots.extend(found);
                discovery_warnings = warnings;
            }
        }
        let active: HashSet<_> = self
            .view
            .read()
            .unwrap()
            .processes
            .iter()
            .map(|p| p.agent.clone())
            .collect();
        let mut result = scan_parallel(&roots, &active, &mut stable, &policy, &progress);
        if auto_discover && !cancelled() {
            if let Some(home) = dirs::home_dir() {
                let processes = self.view.read().unwrap().processes.clone();
                let found =
                    discover_projects(&home, &roots, &result.sessions, &processes, &ignored);
                if !found.is_empty() {
                    let mut extra_stable = HashMap::new();
                    let extra =
                        scan_parallel(&found, &active, &mut extra_stable, &policy, &progress);
                    stable.extend(extra_stable);
                    result.files.extend(extra.files);
                    result.sessions.extend(extra.sessions);
                    result.warnings.extend(extra.warnings);
                    result.complete &= extra.complete;
                    roots.extend(found);
                }
            }
        }
        if cancelled() {
            let mut core = self.core.lock().unwrap();
            core.stable.extend(stable);
            if full {
                self.full_scan.store(true, Ordering::Relaxed);
            }
            self.forced.store(true, Ordering::Relaxed);
            progress.update(true, |p| p.phase = "paused".into());
            return Ok(());
        }
        result
            .files
            .sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.path.cmp(&b.path)));
        result.sessions.sort_by(|a, b| b.modified.cmp(&a.modified));
        apply_session_retention(&mut result.files, &policy);
        let current_active: HashSet<_> = self
            .view
            .read()
            .unwrap()
            .processes
            .iter()
            .map(|p| p.agent.clone())
            .collect();
        for file in &mut result.files {
            if file.eligible && current_active.contains(&file.agent) {
                file.eligible = false;
                file.cleanup_status = "active".into();
                file.reason = "Agent is running; close it before reviewing its files".into();
            }
        }
        progress.update(true, |p| p.phase = "saving".into());
        // A separate SQLite connection lets resource sampling continue while the
        // complete file inventory is committed atomically for paginated inspection.
        let mut inventory_store = Store::open(&data_path)?;
        let committed =
            inventory_store.replace_inventory_checked(&result.files, &roots, &cancelled)?;
        if !committed {
            self.core.lock().unwrap().stable.extend(stable);
            self.forced.store(true, Ordering::Relaxed);
            if full {
                self.full_scan.store(true, Ordering::Relaxed);
            }
            progress.update(true, |p| p.phase = "paused".into());
            return Ok(());
        }
        let mut core = self.core.lock().unwrap();
        if cancelled() {
            self.forced.store(true, Ordering::Relaxed);
            if full {
                self.full_scan.store(true, Ordering::Relaxed);
            }
            progress.update(true, |p| p.phase = "paused".into());
            return Ok(());
        }
        let scope_changed = roots.iter().any(|r| !before_roots.contains(&r.path));
        let mut view = self.view.write().unwrap();
        let count = roots
            .iter()
            .filter(|r| r.custom && !before_roots.contains(&r.path))
            .count();
        let existing = !view.activity.iter().any(|a| a.kind == "discovery");
        for event in activity::discovery_events(&roots, &before_roots, existing) {
            core.store.event(&event)?;
            view.activity.insert(0, event);
        }
        if count > 0 {
            core.baseline.clear();
        }
        if !scope_changed && !core.files.is_empty() && view.scan_complete && result.complete {
            for definition in definitions(&core.custom_agents)
                .into_iter()
                .filter(|d| d.enabled)
            {
                let agent = &definition.id;
                let before: u64 = core
                    .files
                    .values()
                    .filter(|f| f.agent == *agent)
                    .map(|f| f.size)
                    .sum();
                let after: u64 = result
                    .files
                    .iter()
                    .filter(|f| f.agent == *agent)
                    .map(|f| f.size)
                    .sum();
                let delta = after as i64 - before as i64;
                if delta != 0 {
                    let event = Activity {
                        id: uuid::Uuid::new_v4().to_string(),
                        time: now(),
                        kind: if delta > 0 { "growth" } else { "decrease" }.into(),
                        agent: agent.to_string(),
                        title: if delta > 0 {
                            "Storage increased"
                        } else {
                            "Storage decreased"
                        }
                        .into(),
                        detail: "Observed net change in monitored directories".into(),
                        bytes: delta,
                        context: None,
                    };
                    core.store.event(&event)?;
                    view.activity.insert(0, event);
                }
            }
        }
        core.roots = roots.clone();
        core.store.set(
            "roots",
            &roots.iter().filter(|r| r.custom).collect::<Vec<_>>(),
        )?;
        if full {
            core.store.set("full_discovery_version", &1u32)?;
        }
        view.scan_complete = result.complete && discovery_warnings.is_empty();
        view.warnings.retain(|w| w.contains("interrupted"));
        view.warnings.extend(discovery_warnings);
        view.warnings.extend(result.warnings);
        view.warnings.truncate(24);
        view.scanned_at = now();
        view.roots = roots;
        view.file_count = result.files.len();
        view.cleanup_summary = cleanup_summary(result.files.iter());
        view.sessions = result.sessions.into_iter().take(300).collect();
        view.files = result
            .files
            .iter()
            .filter(|f| f.eligible)
            .take(500)
            .cloned()
            .collect();
        let ids: HashSet<_> = view.files.iter().map(|f| f.id.clone()).collect();
        view.files.extend(
            result
                .files
                .iter()
                .filter(|f| !ids.contains(&f.id))
                .take(300)
                .cloned(),
        );
        core.files = result
            .files
            .into_iter()
            .map(|f| (f.id.clone(), f))
            .collect();
        core.stable = stable;
        if view.scan_complete {
            for definition in definitions(&core.custom_agents)
                .into_iter()
                .filter(|d| d.enabled)
            {
                let agent = &definition.id;
                let storage = core
                    .files
                    .values()
                    .filter(|f| f.agent == *agent)
                    .map(|f| f.size)
                    .sum::<u64>();
                core.baseline.entry(agent.to_string()).or_insert(storage);
            }
            core.store.set("baseline", &core.baseline)?;
        }
        core.plans.clear();
        let disks = Disks::new_with_refreshed_list();
        let home = dirs::home_dir().unwrap_or_default();
        if let Some(disk) = disks
            .list()
            .iter()
            .filter(|d| home.starts_with(d.mount_point()))
            .max_by_key(|d| d.mount_point().components().count())
        {
            view.disk_total = disk.total_space();
            view.disk_available = disk.available_space();
        }
        drop(view);
        drop(core);
        progress.update(true, |p| {
            p.phase = "complete".into();
            p.projects = count;
        });
        Ok(())
    }
}
impl Engine {
    fn adopt(&self, request: AdoptRequest) -> Result<Snapshot, String> {
        let mut core = self.core.lock().unwrap();
        let previous = self.view.read().unwrap().processes.clone();
        // Preview is advisory. Revalidate the complete request against fresh processes at commit.
        let preview = core.adoption_preview(&request, &previous)?;
        let id = format!("custom-{}", uuid::Uuid::new_v4());
        let mut definitions = core.custom_agents.clone();
        definitions.push(AgentDefinition {
            id: id.clone(),
            name: request.name.trim().into(),
            color: request.color,
            builtin: false,
            enabled: true,
            rule: preview.rule,
        });
        let mut roots = core.roots.clone();
        roots.extend(preview.roots.into_iter().map(|mut r| {
            r.agent = id.clone();
            r
        }));
        self.revision.fetch_add(1, Ordering::Relaxed);
        core.store.save_registry(&definitions, &roots)?;
        core.custom_agents = definitions;
        core.roots = roots;
        if let (Some(pid), Some(start_time)) = (request.pid, request.start_time) {
            core.bindings.push(AgentBinding {
                agent: id,
                pid,
                start_time,
                executable: request.executable,
            });
        }
        core.plans.clear();
        core.baseline.clear();
        drop(core);
        self.request_scan();
        self.sample()
    }
    fn detach(&self, id: &str) -> Result<Snapshot, String> {
        let mut core = self.core.lock().unwrap();
        let mut definitions = core.custom_agents.clone();
        let definition = definitions
            .iter_mut()
            .find(|d| d.id == id && d.enabled)
            .ok_or("Only active custom Agents can be detached")?;
        definition.enabled = false;
        definition.rule = None;
        let removed: HashSet<_> = core
            .roots
            .iter()
            .filter(|r| r.agent == id)
            .map(|r| r.id.clone())
            .collect();
        let roots = core
            .roots
            .iter()
            .filter(|r| r.agent != id)
            .cloned()
            .collect::<Vec<_>>();
        self.revision.fetch_add(1, Ordering::Relaxed);
        core.store.save_registry(&definitions, &roots)?;
        core.custom_agents = definitions;
        core.roots = roots;
        core.bindings.retain(|b| b.agent != id);
        core.files.retain(|_, f| f.agent != id);
        core.stable.clear();
        core.plans.clear();
        core.baseline.remove(id);
        let mut view = self.view.write().unwrap();
        view.processes.retain(|p| p.agent != id);
        view.files.retain(|f| f.agent != id);
        view.sessions.retain(|s| s.agent != id);
        view.roots.retain(|r| !removed.contains(&r.id));
        view.agents.retain(|a| a.id != id);
        view.file_count = core.files.len();
        view.cleanup_summary = cleanup_summary(core.files.values());
        view.agent_definitions = adoption::definitions(&core.custom_agents);
        drop(view);
        drop(core);
        self.request_scan();
        self.sample()
    }
}
#[tauri::command]
async fn list_process_candidates(
    engine: tauri::State<'_, Engine>,
) -> Result<Vec<ProcessCandidate>, String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut core = engine.core.lock().unwrap();
        let previous = engine.view.read().unwrap().processes.clone();
        let managed = core.collect_processes(&previous);
        Ok(process_candidates(&core.system, &managed))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn preview_adoption(
    engine: tauri::State<'_, Engine>,
    request: AdoptRequest,
) -> Result<AdoptionPreview, String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut core = engine.core.lock().unwrap();
        let previous = engine.view.read().unwrap().processes.clone();
        core.adoption_preview(&request, &previous)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn adopt_agent(
    app: tauri::AppHandle,
    engine: tauri::State<'_, Engine>,
    request: AdoptRequest,
) -> Result<Snapshot, String> {
    let engine = engine.inner().clone();
    let snapshot = tauri::async_runtime::spawn_blocking(move || engine.adopt(request))
        .await
        .map_err(|e| e.to_string())??;
    let _ = app.emit("snapshot", &snapshot);
    Ok(snapshot)
}
#[tauri::command]
async fn detach_agent(
    app: tauri::AppHandle,
    engine: tauri::State<'_, Engine>,
    id: String,
) -> Result<Snapshot, String> {
    let engine = engine.inner().clone();
    let snapshot = tauri::async_runtime::spawn_blocking(move || engine.detach(&id))
        .await
        .map_err(|e| e.to_string())??;
    let _ = app.emit("snapshot", &snapshot);
    Ok(snapshot)
}

#[tauri::command]
async fn resource_history(
    engine: tauri::State<'_, Engine>,
    scope: String,
    id: String,
    seconds: u64,
) -> Result<Vec<resources::ResourcePoint>, String> {
    if !["all", "project", "process"].contains(&scope.as_str())
        || id.len() > 4096
        || ![3600, 86400].contains(&seconds)
    {
        return Err("Invalid resource history filter".into());
    }
    let path = engine.core.lock().unwrap().data_path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        Store::open(&path)?.resource_history(&scope, &id, now().saturating_sub(seconds))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn inventory_details(
    engine: tauri::State<'_, Engine>,
    status: String,
    root: Option<String>,
    query: String,
    offset: usize,
) -> Result<InventoryDetails, String> {
    if ![
        "eligible",
        "active",
        "recent",
        "sessions",
        "protected",
        "stability",
        "unsafe",
        "excluded",
    ]
    .contains(&status.as_str())
        || query.len() > 1024
    {
        return Err("Invalid inventory filter".into());
    }
    let path = engine.core.lock().unwrap().data_path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        Store::open(&path)?.inventory_details(&status, root.as_deref(), &query, offset)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
fn full_rescan(engine: tauri::State<Engine>) {
    engine.full_scan.store(true, Ordering::Relaxed);
    engine.request_scan();
}
#[tauri::command]
fn get_snapshot(engine: tauri::State<Engine>) -> Result<Snapshot, String> {
    engine
        .view
        .read()
        .map(|v| v.clone())
        .map_err(|_| "Snapshot unavailable".into())
}
#[tauri::command]
fn refresh(engine: tauri::State<Engine>) {
    engine.request_scan();
}
#[tauri::command]
async fn refresh_process_analysis(
    app: tauri::AppHandle,
    engine: tauri::State<'_, Engine>,
) -> Result<Snapshot, String> {
    let engine = engine.inner().clone();
    let snapshot = tauri::async_runtime::spawn_blocking(move || engine.sample())
        .await
        .map_err(|error| error.to_string())??;
    let _ = app.emit("snapshot", &snapshot);
    Ok(snapshot)
}
#[tauri::command]
fn set_monitoring(engine: tauri::State<Engine>, enabled: bool) {
    engine.paused.store(!enabled, Ordering::Relaxed);
    if enabled {
        engine.dirty.store(true, Ordering::Relaxed)
    }
    engine.request_scan();
}
#[tauri::command]
fn save_settings(
    _app: tauri::AppHandle,
    engine: tauri::State<Engine>,
    settings: Settings,
) -> Result<(), String> {
    let mut core = engine.core.lock().unwrap();
    if ![2, 5, 10, 30].contains(&settings.interval)
        || ![1, 7, 30].contains(&settings.retention_days)
        || !["zh-CN", "en-US"].contains(&settings.language.as_str())
        || !["light", "dark", "custom"].contains(&settings.theme.as_str())
        || !["light", "dark"].contains(&settings.custom_base.as_str())
        || settings.custom_accent.len() != 7
        || !settings.custom_accent.starts_with('#')
        || !settings.custom_accent[1..]
            .bytes()
            .all(|b| b.is_ascii_hexdigit())
        || !["conservative", "balanced", "custom"].contains(&settings.cleanup.preset.as_str())
        || !(1..=720).contains(&settings.cleanup.temporary_hours)
        || !(1..=365).contains(&settings.cleanup.cache_days)
        || !(1..=365).contains(&settings.cleanup.log_days)
        || !(7..=365).contains(&settings.cleanup.session_days)
        || !(1..=365).contains(&settings.cleanup.session_cache_days)
        || !["all", "archived"].contains(&settings.cleanup.session_scope.as_str())
        || settings.cleanup.keep_latest_sessions > 1000
        || settings.cleanup.minimum_size_mb > 10000
        || settings.cleanup.excluded_paths.len() > 100
        || settings.cleanup.excluded_paths.iter().any(|p| {
            p.len() > 4096
                || !Path::new(p).is_absolute()
                || Path::new(p)
                    .components()
                    .any(|c| matches!(c, std::path::Component::ParentDir))
        })
        || settings
            .cleanup
            .excluded_agents
            .iter()
            .any(|id| !definitions(&core.custom_agents).iter().any(|d| d.id == *id))
    {
        return Err("Invalid preferences".into());
    }
    core.store.set("settings", &settings)?;
    #[cfg(target_os = "macos")]
    if core.settings.language != settings.language {
        apply_app_menu(&_app, &settings.language).map_err(|e| e.to_string())?;
    }
    let affects_inventory = core.settings.cleanup != settings.cleanup
        || core.settings.auto_discover != settings.auto_discover;
    if affects_inventory {
        engine.revision.fetch_add(1, Ordering::Relaxed);
        for file in core.files.values_mut() {
            file.eligible = false;
        }
    }
    core.settings = settings;
    core.plans.clear();
    drop(core);
    if affects_inventory {
        engine.request_scan();
    }
    Ok(())
}
#[tauri::command]
async fn add_root(
    engine: tauri::State<'_, Engine>,
    path: String,
    agent: String,
) -> Result<(), String> {
    let mut core = engine.core.lock().unwrap();
    if !definitions(&core.custom_agents)
        .iter()
        .any(|d| d.id == agent && d.enabled)
    {
        return Err("Choose a supported agent".into());
    }
    let selected = PathBuf::from(&path);
    if !selected.is_absolute() || selected.is_symlink() || !selected.is_dir() {
        return Err("Select an existing real directory".into());
    }
    let canonical = fs::canonicalize(&selected).map_err(|e| e.to_string())?;
    let home = dirs::home_dir().ok_or("Home unavailable")?;
    if canonical == home || home.starts_with(&canonical) || !canonical.starts_with(&home) {
        return Err("Select a project directory inside your home, rather than a whole disk or home directory".into());
    }
    if core
        .roots
        .iter()
        .any(|r| canonical.starts_with(&r.path) || Path::new(&r.path).starts_with(&canonical))
    {
        return Err("This directory overlaps an existing monitoring root".into());
    }
    core.baseline.clear();
    engine.revision.fetch_add(1, Ordering::Relaxed);
    core.roots.push(Root {
        id: uuid::Uuid::new_v4().to_string(),
        label: canonical
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        path: canonical.to_string_lossy().to_string(),
        agent,
        custom: true,
        source: "manual".into(),
    });
    core.store.set(
        "roots",
        &core.roots.iter().filter(|r| r.custom).collect::<Vec<_>>(),
    )?;
    engine.dirty.store(true, Ordering::Relaxed);
    engine.request_scan();
    Ok(())
}
#[tauri::command]
fn remove_root(engine: tauri::State<Engine>, id: String) -> Result<(), String> {
    let mut core = engine.core.lock().unwrap();
    if !core.roots.iter().any(|r| r.id == id && r.custom) {
        return Err("Only user-added directories can be removed".into());
    }
    if let Some(root) = core.roots.iter().find(|r| r.id == id) {
        let path = root.path.clone();
        core.ignored_roots.insert(path);
        core.store.set("ignored_roots", &core.ignored_roots)?;
    }
    engine.revision.fetch_add(1, Ordering::Relaxed);
    core.roots.retain(|r| r.id != id);
    core.baseline.clear();
    core.store.set(
        "roots",
        &core.roots.iter().filter(|r| r.custom).collect::<Vec<_>>(),
    )?;
    engine.dirty.store(true, Ordering::Relaxed);
    engine.request_scan();
    Ok(())
}
fn ancestry(path: &Path, root: &Path) -> Result<HashMap<String, String>, String> {
    if !path_is_safe(path, root) {
        return Err("Unsafe or changed path".into());
    }
    let mut result = HashMap::new();
    let mut p = path.parent();
    while let Some(parent) = p {
        let meta = fs::symlink_metadata(parent).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        let identity = {
            use std::os::unix::fs::MetadataExt;
            format!("{}:{}", meta.dev(), meta.ino())
        };
        #[cfg(not(unix))]
        let identity = windows_identity(parent)
            .ok_or("Unable to verify directory identity")?
            .0;
        result.insert(parent.to_string_lossy().to_string(), identity);
        if parent == root {
            break;
        }
        p = parent.parent();
    }
    Ok(result)
}
#[tauri::command]
fn prepare_cleanup(engine: tauri::State<Engine>, ids: Vec<String>) -> Result<CleanupPlan, String> {
    prepare_plan(engine.inner(), ids)
}
fn prepare_plan(engine: &Engine, ids: Vec<String>) -> Result<CleanupPlan, String> {
    if engine.paused.load(Ordering::Relaxed) {
        return Err("Resume monitoring before preparing cleanup".into());
    }
    if ids.is_empty() || ids.len() > 500 {
        return Err("Select 1–500 candidates".into());
    }
    let previous = engine.view.read().unwrap().processes.clone();
    let mut core = engine.core.lock().unwrap();
    let mut items = vec![];
    let mut ancestors = HashMap::new();
    let active: HashSet<_> = core
        .collect_processes(&previous)
        .iter()
        .map(|p| p.agent.clone())
        .collect();
    for id in ids.into_iter().collect::<HashSet<_>>() {
        let f = core.files.get(&id).ok_or("Candidate no longer available")?;
        if !f.eligible || active.contains(&f.agent) {
            return Err(
                "A selected file is protected or its agent is running. Refresh the list.".into(),
            );
        }
        let root = core
            .roots
            .iter()
            .find(|r| r.id == f.root_id)
            .ok_or("Root unavailable")?;
        if cleanup_exclusion(f, &core.settings.cleanup).is_some()
            || !cleanup_age(Path::new(&f.relative_path), root, &core.settings.cleanup)
                .is_some_and(|age| now().saturating_sub(f.modified) >= age)
        {
            return Err("Cleanup policy changed; scan and review again".into());
        }
        ancestors.extend(ancestry(Path::new(&f.path), Path::new(&root.path))?);
        let meta = fs::symlink_metadata(&f.path).map_err(|e| e.to_string())?;
        if !meta.is_file()
            || !links_safe(Path::new(&f.path), &meta)
            || fingerprint(Path::new(&f.path), &meta) != f.identity
        {
            return Err("A selected file changed. Scan again.".into());
        }
        items.push(f.clone());
    }
    core.plans.retain(|_, (p, _)| p.expires_at > now());
    let plan = CleanupPlan {
        id: uuid::Uuid::new_v4().to_string(),
        expires_at: now() + 60,
        total: items.iter().map(|f| f.size).sum(),
        items,
    };
    core.plans
        .insert(plan.id.clone(), (plan.clone(), ancestors));
    Ok(plan)
}
#[tauri::command]
async fn execute_cleanup(
    engine: tauri::State<'_, Engine>,
    plan_id: String,
) -> Result<ActionResult, String> {
    let engine = engine.inner().clone();
    tauri::async_runtime::spawn_blocking(move || execute_plan(&engine, plan_id))
        .await
        .map_err(|e| e.to_string())?
}
fn execute_plan(engine: &Engine, plan_id: String) -> Result<ActionResult, String> {
    execute_plan_with(engine, plan_id, |path| {
        trash::delete(path).map_err(|e| e.to_string())
    })
}
fn execute_plan_with(
    engine: &Engine,
    plan_id: String,
    mut mover: impl FnMut(&str) -> Result<(), String>,
) -> Result<ActionResult, String> {
    let mut core = engine.core.lock().unwrap();
    let (plan, ancestors) = core
        .plans
        .remove(&plan_id)
        .ok_or("Plan expired or already executed")?;
    if plan.expires_at < now() {
        return Err("Cleanup plan expired. Review the files again.".into());
    }
    let previous = engine.view.read().unwrap().processes.clone();
    let active: HashSet<_> = core
        .collect_processes(&previous)
        .iter()
        .map(|p| p.agent.clone())
        .collect();
    let mut result = ActionResult {
        succeeded: 0,
        bytes: 0,
        failures: vec![],
    };
    for file in plan.items {
        let attempt = (|| -> Result<(), String> {
            if active.contains(&file.agent) {
                return Err("Agent is running".into());
            }
            let root = core
                .roots
                .iter()
                .find(|r| r.id == file.root_id)
                .ok_or("Root removed")?;
            if cleanup_exclusion(&file, &core.settings.cleanup).is_some()
                || !cleanup_age(Path::new(&file.relative_path), root, &core.settings.cleanup)
                    .is_some_and(|age| now().saturating_sub(file.modified) >= age)
            {
                return Err("Cleanup policy changed; scan and review again".into());
            }
            let checked = ancestry(Path::new(&file.path), Path::new(&root.path))?;
            if checked.iter().any(|(p, i)| ancestors.get(p) != Some(i)) {
                return Err("Ancestor directory changed".into());
            }
            let meta = fs::symlink_metadata(&file.path).map_err(|e| e.to_string())?;
            if !meta.is_file()
                || !links_safe(Path::new(&file.path), &meta)
                || fingerprint(Path::new(&file.path), &meta) != file.identity
            {
                return Err("File changed since review".into());
            }
            let operation = format!("{}:{}", plan.id, file.id);
            core.store
                .journal(&operation, "intent", &serde_json::to_string(&file).unwrap())?;
            match mover(&file.path) {
                Ok(()) => {
                    core.store.journal(
                        &operation,
                        "moved",
                        &serde_json::to_string(&file).unwrap(),
                    )?;
                    Ok(())
                }
                Err(e) => {
                    core.store.journal(&operation, "failed", &e.to_string())?;
                    Err(e.to_string())
                }
            }
        })();
        match attempt {
            Ok(()) => {
                result.succeeded += 1;
                result.bytes += file.size
            }
            Err(e) => result
                .failures
                .push(format!("{}: {}", file.relative_path, e)),
        }
    }
    let event = Activity {
        id: uuid::Uuid::new_v4().to_string(),
        time: now(),
        kind: "cleanup".into(),
        agent: "system".into(),
        title: format!("{} files moved to Trash", result.succeeded),
        detail: "Restore using the system Trash. Disk space is released when Trash is emptied."
            .into(),
        bytes: -(result.bytes as i64),
        context: Some(ActivityContext {
            action: "cleanup".into(),
            count: result.succeeded,
            failed: result.failures.len(),
            paths: vec![],
            ..Default::default()
        }),
    };
    core.store.event(&event)?;
    engine.view.write().unwrap().activity.insert(0, event);
    engine.dirty.store(true, Ordering::Relaxed);
    Ok(result)
}
#[tauri::command]
fn stop_process(engine: tauri::State<Engine>, pid: u32, start_time: u64) -> Result<(), String> {
    request_stop(engine.inner(), pid, start_time)
}
#[tauri::command]
fn stop_suggested_process(
    engine: tauri::State<Engine>,
    pid: u32,
    start_time: u64,
) -> Result<(), String> {
    request_stop_checked(engine.inner(), pid, start_time, true)
}
fn request_stop(engine: &Engine, pid: u32, start_time: u64) -> Result<(), String> {
    request_stop_checked(engine, pid, start_time, false)
}
fn request_stop_checked(
    engine: &Engine,
    pid: u32,
    start_time: u64,
    suggested_only: bool,
) -> Result<(), String> {
    let mut core = engine.core.lock().unwrap();
    let previous = engine.view.read().unwrap().processes.clone();
    let current = core.collect_processes(&previous);
    let p = current
        .iter()
        .find(|p| p.pid == pid && p.start_time == start_time)
        .ok_or("Process exited or its identity changed")?;
    if suggested_only && p.assessment.category != "suggested" {
        return Err("Process activity changed; this task is no longer a cleanup suggestion".into());
    }
    // Protect this app and every ancestor (including the agent launching this development session).
    let mut ancestor = sysinfo::get_current_pid().ok();
    for _ in 0..64 {
        if let Some(a) = ancestor {
            if a.as_u32() == pid {
                return Err("Cannot stop AgentLens or a process that launched it".into());
            }
            ancestor = core.system.process(a).and_then(|p| p.parent())
        } else {
            break;
        }
    }
    let process = core
        .system
        .process(Pid::from_u32(pid))
        .ok_or("Process exited")?;
    if process.start_time() != p.start_time {
        return Err("Process identity changed".into());
    }
    #[cfg(unix)]
    let ok = process.kill_with(Signal::Term).unwrap_or(false);
    #[cfg(not(unix))]
    let ok = process.kill();
    if !ok {
        return Err("System denied the termination request".into());
    }
    let event = Activity {
        id: uuid::Uuid::new_v4().to_string(),
        time: now(),
        kind: "process".into(),
        agent: p.agent.clone(),
        title: format!("Termination requested for {}", p.name),
        detail: format!("PID {} · user-confirmed process action", pid),
        bytes: 0,
        context: Some(ActivityContext {
            action: "stop_requested".into(),
            count: 1,
            processes: vec![p.clone()],
            ..Default::default()
        }),
    };
    core.store.event(&event)?;
    engine.view.write().unwrap().activity.insert(0, event);
    Ok(())
}

#[tauri::command]
fn hide_quicklook(app: tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("quicklook") {
        let _ = w.hide();
    }
}
#[tauri::command]
fn open_main(app: tauri::AppHandle, page: String) -> Result<(), String> {
    if !["Overview", "Cleanup", "Settings"].contains(&page.as_str()) {
        return Err("Unknown page".into());
    }
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        let _ = w.emit("navigate", page);
    }
    hide_quicklook(app);
    Ok(())
}
fn show_quicklook(app: &tauri::AppHandle, rect: tauri::Rect) {
    if let Some(w) = app.get_webview_window("quicklook") {
        let fallback_scale = w.scale_factor().unwrap_or(1.0);
        let pos = rect.position.to_physical::<f64>(fallback_scale);
        let size = rect.size.to_physical::<f64>(fallback_scale);
        // The hidden popover can still belong to another display. Anchor to
        // the tray's display so a monitor above the main screen cannot pull it away.
        let monitor = app.available_monitors().ok().and_then(|monitors| {
            monitors.into_iter().find(|m| {
                let origin = m.position();
                let limit = m.size();
                let center_x = pos.x + size.width / 2.0;
                let center_y = pos.y + size.height / 2.0;
                center_x >= origin.x as f64
                    && center_x < origin.x as f64 + limit.width as f64
                    && center_y >= origin.y as f64
                    && center_y < origin.y as f64 + limit.height as f64
            })
        });
        let scale = monitor
            .as_ref()
            .map_or(fallback_scale, |m| m.scale_factor());
        let mut x = pos.x + size.width / 2.0 - 172.0 * scale;
        let mut y = pos.y + size.height + 6.0 * scale;
        if let Some(m) = monitor {
            let origin = m.position();
            let limit = m.size();
            x = x
                .max(origin.x as f64 + 8.0 * scale)
                .min((origin.x as f64 + limit.width as f64 - 352.0 * scale).max(origin.x as f64));
            if y + 372.0 * scale > origin.y as f64 + limit.height as f64 {
                y = pos.y - 378.0 * scale;
            }
        }
        let _ = w.set_position(tauri::PhysicalPosition::new(x, y));
        let _ = w.set_size(tauri::LogicalSize::new(344.0, 372.0));
        let _ = w.show();
        let _ = w.set_focus();
    }
}
#[cfg(target_os = "macos")]
fn apply_app_menu(app: &tauri::AppHandle, language: &str) -> tauri::Result<()> {
    use tauri::menu::{AboutMetadata, PredefinedMenuItem as Item, Submenu};
    let chinese = language == "zh-CN";
    let text = |zh: &'static str, en: &'static str| if chinese { zh } else { en };
    let application = Submenu::with_items(
        app,
        "AgentLens",
        true,
        &[
            &Item::about(
                app,
                Some(text("关于 AgentLens", "About AgentLens")),
                Some(AboutMetadata {
                    name: Some("AgentLens".into()),
                    version: Some(env!("CARGO_PKG_VERSION").into()),
                    ..Default::default()
                }),
            )?,
            &Item::separator(app)?,
            &MenuItem::with_id(
                app,
                "preferences",
                text("设置…", "Settings…"),
                true,
                Some("CmdOrCtrl+,"),
            )?,
            &Item::separator(app)?,
            &Item::hide(app, Some(text("隐藏 AgentLens", "Hide AgentLens")))?,
            &Item::hide_others(app, Some(text("隐藏其他应用", "Hide Others")))?,
            &Item::separator(app)?,
            &Item::quit(app, Some(text("退出 AgentLens", "Quit AgentLens")))?,
        ],
    )?;
    let edit = Submenu::with_items(
        app,
        text("编辑", "Edit"),
        true,
        &[
            &Item::undo(app, Some(text("撤销", "Undo")))?,
            &Item::redo(app, Some(text("重做", "Redo")))?,
            &Item::separator(app)?,
            &Item::cut(app, Some(text("剪切", "Cut")))?,
            &Item::copy(app, Some(text("复制", "Copy")))?,
            &Item::paste(app, Some(text("粘贴", "Paste")))?,
            &Item::select_all(app, Some(text("全选", "Select All")))?,
        ],
    )?;
    let window = Submenu::with_items(
        app,
        text("窗口", "Window"),
        true,
        &[
            &Item::minimize(app, Some(text("最小化", "Minimize")))?,
            &Item::maximize(app, Some(text("缩放", "Zoom")))?,
            &Item::fullscreen(app, Some(text("进入全屏幕", "Enter Full Screen")))?,
            &Item::separator(app)?,
            &Item::close_window(app, Some(text("关闭窗口", "Close Window")))?,
        ],
    )?;
    app.set_menu(Menu::with_items(app, &[&application, &edit, &window])?)?;
    Ok(())
}
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let engine = Engine::new(&app.path().app_data_dir()?).map_err(std::io::Error::other)?;
            app.manage(engine.clone());
            #[cfg(target_os = "macos")]
            apply_app_menu(app.handle(), &engine.view.read().unwrap().settings.language)?;
            app.on_menu_event(|app, event| {
                if event.id.as_ref() == "preferences" {
                    let _ = open_main(app.clone(), "Settings".into());
                }
            });
            let window = app.get_webview_window("main").unwrap();
            tauri::WebviewWindowBuilder::new(
                app,
                "quicklook",
                tauri::WebviewUrl::App("index.html?tray=1".into()),
            )
            .title("AgentLens Quick Look")
            .inner_size(344.0, 372.0)
            .decorations(false)
            .transparent(true)
            .resizable(false)
            .always_on_top(true)
            .accept_first_mouse(true)
            .skip_taskbar(true)
            .visible(false)
            .focused(false)
            .build()?;
            #[cfg(target_os = "macos")]
            if window_vibrancy::apply_liquid_glass(
                &window,
                window_vibrancy::LiquidGlassOptions::default().radius(14.0),
            )
            .is_err()
            {
                let _ = window_vibrancy::apply_vibrancy(
                    &window,
                    window_vibrancy::NSVisualEffectMaterial::Sidebar,
                    None,
                    Some(14.0),
                );
            }
            #[cfg(target_os = "windows")]
            {
                let _ = window_vibrancy::apply_mica(&window, Some(false));
            }
            let open = MenuItem::with_id(app, "open", "打开 AgentLens", true, None::<&str>)?;
            let pause = MenuItem::with_id(app, "pause", "暂停 / 继续监测", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出 AgentLens", true, None::<&str>)?;
            let memory_item = MenuItem::with_id(app, "ram", "正在读取资源…", false, None::<&str>)?;
            let storage_item =
                MenuItem::with_id(app, "storage", "正在读取存储…", false, None::<&str>)?;
            let activity_item =
                MenuItem::with_id(app, "active", "正在读取进程…", false, None::<&str>)?;
            let separator = tauri::menu::PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(
                app,
                &[
                    &memory_item,
                    &storage_item,
                    &activity_item,
                    &separator,
                    &open,
                    &pause,
                    &quit,
                ],
            )?;
            let tray = TrayIconBuilder::with_id("monitor")
                .icon(tauri::image::Image::from_bytes(include_bytes!(
                    "../icons/tray.png"
                ))?)
                .icon_as_template(true)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_tray_icon_event(|tray, event| {
                    let app = tray.app_handle();
                    match event {
                        tauri::tray::TrayIconEvent::Click {
                            rect,
                            button: tauri::tray::MouseButton::Left,
                            button_state: tauri::tray::MouseButtonState::Up,
                            ..
                        } => {
                            let visible = app
                                .get_webview_window("quicklook")
                                .is_some_and(|window| window.is_visible().unwrap_or(false));
                            if visible {
                                hide_quicklook(app.clone());
                            } else {
                                show_quicklook(app, rect);
                            }
                        }
                        _ => {}
                    }
                })
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.unminimize();
                            let _ = w.set_focus();
                        }
                    }
                    "pause" => {
                        let e = app.state::<Engine>();
                        let old = e.paused.load(Ordering::Relaxed);
                        e.paused.store(!old, Ordering::Relaxed);
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            let handle = app.handle().clone();
            let worker = engine.clone();
            std::thread::Builder::new()
                .name("agentlens-monitor".into())
                .spawn(move || {
                    let dirty = worker.dirty.clone();
                    let mut watcher = notify::recommended_watcher(
                        move |result: notify::Result<notify::Event>| {
                            // Reads by the inventory must never schedule another inventory.
                            if result.is_ok_and(|event| {
                                !matches!(event.kind, notify::EventKind::Access(_))
                            }) {
                                dirty.store(true, Ordering::Relaxed)
                            }
                        },
                    )
                    .ok();
                    let mut watched = HashSet::new();
                    loop {
                        if let Ok(core) = worker.core.lock() {
                            if let Some(w) = watcher.as_mut() {
                                for root in &core.roots {
                                    if watched.insert(root.path.clone()) {
                                        let _ = w.watch(
                                            Path::new(&root.path),
                                            notify::RecursiveMode::Recursive,
                                        );
                                    }
                                }
                            }
                        }
                        worker.maybe_scan(handle.clone());
                        match worker.sample() {
                            Ok(snapshot) => {
                                let memory: u64 = snapshot.agents.iter().map(|a| a.memory).sum();
                                let cpu: f32 = snapshot.agents.iter().map(|a| a.cpu).sum();
                                let storage: u64 = snapshot.agents.iter().map(|a| a.storage).sum();
                                let active =
                                    snapshot.agents.iter().filter(|a| a.processes > 0).count();
                                let growth: i64 = snapshot.agents.iter().map(|a| a.growth).sum();
                                let chinese = snapshot.settings.language == "zh-CN";
                                let arrow = if !snapshot.monitoring {
                                    "Ⅱ"
                                } else if snapshot.memory_delta > 0 {
                                    "↑"
                                } else if snapshot.memory_delta < 0 {
                                    "↓"
                                } else {
                                    "−"
                                };
                                #[cfg(not(target_os = "macos"))]
                                let _ = tray.set_tooltip(Some(format!(
                                    "AgentLens · {:.2} GB RAM ({:+.1} MB) · CPU {:.1}% ({:+.1})",
                                    memory as f64 / 1e9,
                                    snapshot.memory_delta as f64 / 1e6,
                                    cpu,
                                    snapshot.cpu_delta
                                )));
                                let _ = open.set_text(if chinese {
                                    "打开 AgentLens"
                                } else {
                                    "Open AgentLens"
                                });
                                let _ = pause.set_text(if chinese {
                                    "暂停 / 继续监测"
                                } else {
                                    "Pause / resume monitoring"
                                });
                                let _ = quit.set_text(if chinese {
                                    "退出 AgentLens"
                                } else {
                                    "Quit AgentLens"
                                });
                                let _ = memory_item.set_text(format!(
                                    "{}: {:.2} GB {} · CPU {:.1}% ({:+.1})",
                                    if chinese { "内存" } else { "RAM" },
                                    memory as f64 / 1e9,
                                    arrow,
                                    cpu,
                                    snapshot.cpu_delta
                                ));
                                let _ = storage_item.set_text(format!(
                                    "{}: {:.2} GB · {:+.1} MB",
                                    if chinese { "存储" } else { "Storage" },
                                    storage as f64 / 1e9,
                                    growth as f64 / 1e6
                                ));
                                let _ = activity_item.set_text(if chinese {
                                    format!(
                                        "{} 个活跃 Agent · {} 个进程",
                                        active,
                                        snapshot.processes.len()
                                    )
                                } else {
                                    format!(
                                        "{} active agents · {} processes",
                                        active,
                                        snapshot.processes.len()
                                    )
                                });
                                #[cfg(target_os = "macos")]
                                {
                                    let _ = tray.set_title(if snapshot.settings.menu_bar {
                                        Some(format!(
                                            "{:.1}G {} · {:.0}%",
                                            memory as f64 / 1e9,
                                            arrow,
                                            cpu
                                        ))
                                    } else {
                                        None
                                    });
                                }
                                let _ = handle.emit("snapshot", &snapshot);
                            }
                            Err(error) => {
                                let mut view = worker.view.write().unwrap();
                                view.scanning = false;
                                let note = format!("Monitoring issue: {}", error);
                                if !view.warnings.contains(&note) {
                                    view.warnings.insert(0, note);
                                    view.warnings.truncate(20);
                                }
                                let _ = handle.emit("snapshot", &*view);
                            }
                        }
                        let interval = worker.view.read().unwrap().settings.interval;
                        let (lock, signal) = &*worker.wake;
                        let (mut pending, _) = signal
                            .wait_timeout_while(
                                lock.lock().unwrap(),
                                Duration::from_secs(interval),
                                |pending| !*pending,
                            )
                            .unwrap();
                        *pending = false;
                    }
                })?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "quicklook" && matches!(event, tauri::WindowEvent::Focused(false))
            {
                hide_quicklook(window.app_handle().clone());
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_snapshot,
            resource_history,
            list_process_candidates,
            preview_adoption,
            adopt_agent,
            detach_agent,
            inventory_details,
            full_rescan,
            refresh,
            refresh_process_analysis,
            set_monitoring,
            save_settings,
            add_root,
            remove_root,
            prepare_cleanup,
            execute_cleanup,
            stop_process,
            stop_suggested_process,
            hide_quicklook,
            open_main
        ])
        .run(tauri::generate_context!())
        .expect("Unable to start AgentLens");
}

/// Collect real metadata into an isolated history directory. No cleanup is performed.
pub fn probe(path: &Path) -> Result<serde_json::Value, String> {
    let engine = Engine::new(path)?;
    let started = std::time::Instant::now();
    engine.sample()?;
    engine.run_inventory(true, &|_| {})?;
    let snapshot = engine.sample()?;
    Ok(serde_json::json!({
        "platform": snapshot.platform,
        "files": snapshot.file_count,
        "roots": snapshot.roots.len(),
        "processes": snapshot.processes.len(),
        "sessions": snapshot.sessions.len(),
        "systemMemory": snapshot.total_memory,
        "usedMemory": snapshot.used_memory,
        "scanComplete": snapshot.scan_complete,
        "warnings": snapshot.warnings,
        "elapsedMs": started.elapsed().as_millis(),
        "agents": snapshot.agents,
    }))
}

#[cfg(test)]
mod safety_tests {
    use super::*;
    fn fixture() -> (tempfile::TempDir, Engine, String) {
        let temp = tempfile::tempdir().unwrap();
        let watched = temp.path().join("cache");
        fs::create_dir_all(watched.join("tmp")).unwrap();
        let path = watched.join("tmp/old.bin");
        fs::write(&path, b"fixture only").unwrap();
        let engine = Engine::new(&temp.path().join("data")).unwrap();
        let root = Root {
            id: "fixture-root".into(),
            path: watched.to_string_lossy().into(),
            agent: "test-fixture".into(),
            label: "Fixture".into(),
            custom: false,
            source: "manual".into(),
        };
        let meta = fs::symlink_metadata(&path).unwrap();
        let id = path.to_string_lossy().to_string();
        let file = FileEntry {
            id: id.clone(),
            root_id: root.id.clone(),
            path: id.clone(),
            relative_path: "tmp/old.bin".into(),
            agent: "test-fixture".into(),
            category: "temporary".into(),
            size: meta.len(),
            modified: now() - 172800,
            identity: fingerprint(&path, &meta),
            eligible: true,
            reason: "Isolated test candidate".into(),
            cleanup_status: "eligible".into(),
            session_id: None,
        };
        {
            let mut c = engine.core.lock().unwrap();
            c.settings.cleanup.cache_days = 1;
            c.roots = vec![root];
            c.files = HashMap::from([(id.clone(), file)]);
        }
        (temp, engine, id)
    }
    #[test]
    fn changed_file_is_skipped_and_plan_cannot_be_replayed() {
        let (_temp, engine, id) = fixture();
        let plan = prepare_plan(&engine, vec![id.clone()]).unwrap();
        fs::write(&id, b"new valuable content").unwrap();
        let result = execute_plan_with(&engine, plan.id.clone(), |_| {
            panic!("Changed file must never reach the mover")
        })
        .unwrap();
        assert_eq!(result.succeeded, 0);
        assert_eq!(result.failures.len(), 1);
        assert!(Path::new(&id).exists());
        assert!(execute_plan_with(&engine, plan.id, |_| Ok(())).is_err());
    }
    #[test]
    fn expired_plan_never_moves_files() {
        let (_temp, engine, id) = fixture();
        let plan = prepare_plan(&engine, vec![id.clone()]).unwrap();
        engine
            .core
            .lock()
            .unwrap()
            .plans
            .get_mut(&plan.id)
            .unwrap()
            .0
            .expires_at = now() - 1;
        assert!(execute_plan_with(&engine, plan.id, |_| panic!(
            "Expired plan must not execute"
        ))
        .is_err());
        assert!(Path::new(&id).exists());
    }
    #[test]
    fn replaced_ancestor_is_rejected_even_when_file_is_unchanged() {
        let (temp, engine, id) = fixture();
        let plan = prepare_plan(&engine, vec![id.clone()]).unwrap();
        let parent = Path::new(&id).parent().unwrap();
        let moved = temp.path().join("original-directory");
        fs::rename(parent, &moved).unwrap();
        fs::create_dir(parent).unwrap();
        fs::rename(moved.join("old.bin"), &id).unwrap();
        let result = execute_plan_with(&engine, plan.id, |_| {
            panic!("Replaced directory must not reach the mover")
        })
        .unwrap();
        assert_eq!(result.succeeded, 0);
        assert!(result.failures[0].contains("Ancestor directory changed"));
    }
    #[test]
    fn successful_fixture_move_is_journaled_and_counted_once() {
        let (temp, engine, id) = fixture();
        let plan = prepare_plan(&engine, vec![id.clone(), id.clone()]).unwrap();
        assert_eq!(plan.items.len(), 1);
        let destination = temp.path().join("mock-trash.bin");
        let result = execute_plan_with(&engine, plan.id, |path| {
            fs::rename(path, &destination).map_err(|e| e.to_string())
        })
        .unwrap();
        assert_eq!(result.succeeded, 1);
        assert_eq!(result.bytes, b"fixture only".len() as u64);
        assert!(!Path::new(&id).exists());
        assert!(destination.exists());
        assert!(!engine.core.lock().unwrap().store.interrupted());
    }
    #[test]
    fn move_failure_is_reported_without_delete_fallback() {
        let (_temp, engine, id) = fixture();
        let plan = prepare_plan(&engine, vec![id.clone()]).unwrap();
        let result =
            execute_plan_with(&engine, plan.id, |_| Err("Trash unavailable".into())).unwrap();
        assert_eq!(result.succeeded, 0);
        assert_eq!(result.bytes, 0);
        assert!(Path::new(&id).exists());
        assert!(result.failures[0].contains("Trash unavailable"));
    }
    #[test]
    fn unknown_candidates_and_changed_process_identity_are_rejected() {
        let (_temp, engine, _id) = fixture();
        assert!(prepare_plan(&engine, vec!["/unknown".into()]).is_err());
        assert!(request_stop(&engine, std::process::id(), 0).is_err());
    }
}

#[cfg(test)]
mod inventory_engine_tests {
    use super::*;
    #[test]
    fn inventory_worker_releases_resource_lock_and_preserves_files_when_cancelled() {
        let directory = tempfile::tempdir().unwrap();
        let project = directory.path().join("project");
        fs::create_dir(&project).unwrap();
        fs::write(project.join("source.rs"), "source").unwrap();
        let engine = Engine::new(&directory.path().join("data")).unwrap();
        {
            let mut core = engine.core.lock().unwrap();
            core.store.set("full_discovery_version", &1u32).unwrap();
            core.settings.auto_discover = false;
            core.roots = vec![Root {
                id: "project".into(),
                path: project.to_string_lossy().into_owned(),
                label: "project".into(),
                agent: "unknown".into(),
                custom: true,
                source: "scan".into(),
            }];
        }
        engine.full_scan.store(false, Ordering::Relaxed);
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let background = engine.clone();
        let gate_worker = gate.clone();
        let sent = AtomicBool::new(false);
        let worker = std::thread::spawn(move || {
            background.run_inventory(true, &|_| {
                if !sent.swap(true, Ordering::Relaxed) {
                    started_tx.send(()).unwrap();
                    let (lock, wake) = &*gate_worker;
                    drop(
                        wake.wait_while(lock.lock().unwrap(), |open| !*open)
                            .unwrap(),
                    );
                }
            })
        });
        started_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        assert!(engine.core.try_lock().is_ok());
        assert!(engine.sample().unwrap().total_memory > 0);
        engine.paused.store(true, Ordering::Relaxed);
        {
            let (lock, wake) = &*gate;
            *lock.lock().unwrap() = true;
            wake.notify_all();
        }
        worker.join().unwrap().unwrap();
        assert!(engine.core.lock().unwrap().files.is_empty());
        assert_eq!(engine.view.read().unwrap().scanned_at, 0);
        engine.paused.store(false, Ordering::Relaxed);
        engine.run_inventory(true, &|_| {}).unwrap();
        let snapshot = engine.sample().unwrap();
        assert_eq!(snapshot.file_count, 1);
        assert!(snapshot.scan_complete);
        assert_eq!(snapshot.cleanup_summary[0].bytes, 6);
        let core = engine.core.lock().unwrap();
        assert_eq!(
            core.store
                .inventory_details("protected", None, "", 0)
                .unwrap()
                .count,
            1
        );
    }
}

#[cfg(all(test, unix))]
mod adoption_engine_tests {
    use super::*;
    use std::process::Command;

    struct FixtureProcesses {
        executables: Vec<PathBuf>,
    }
    impl Drop for FixtureProcesses {
        fn drop(&mut self) {
            let system = System::new_all();
            for process in system.processes().values() {
                if process
                    .exe()
                    .is_some_and(|p| self.executables.iter().any(|e| e == p))
                {
                    let _ = process.kill_with(Signal::Term);
                }
            }
        }
    }
    #[test]
    fn adoption_tracks_trees_persists_rules_and_detaches_without_touching_files_or_processes() {
        let home = dirs::home_dir().unwrap();
        let directory = tempfile::Builder::new()
            .prefix(".agentlens-adoption-test-")
            .tempdir_in(home)
            .unwrap();
        let shell = directory.path().join("fixture-agent");
        let worker = directory.path().join("fixture-worker");
        let source = directory.path().join("fixture.rs");
        fs::write(&source,r#"fn main() {
            let args: Vec<_> = std::env::args().collect();
            if args.get(1).map(String::as_str) == Some("--grandchild") {
                std::thread::sleep(std::time::Duration::from_secs(90));
            } else if args.get(1).map(String::as_str) == Some("--worker") {
                let control = std::env::current_exe().unwrap().parent().unwrap().join("spawn-grandchild");
                while !control.exists() { std::thread::sleep(std::time::Duration::from_millis(10)); }
                let mut child = std::process::Command::new(std::env::current_exe().unwrap()).arg("--grandchild").spawn().unwrap();
                let _ = child.wait();
            } else {
                let mut child = std::process::Command::new(&args[1]).arg("--worker").spawn().unwrap();
                let _ = child.wait();
            }
        }"#).unwrap();
        assert!(Command::new("rustc")
            .arg(&source)
            .arg("-o")
            .arg(&shell)
            .status()
            .unwrap()
            .success());
        fs::copy(&shell, &worker).unwrap();
        let _guard = FixtureProcesses {
            executables: vec![shell.clone(), worker.clone()],
        };
        // Reparent the fixture so it is not mistaken for AgentLens's own helper tree.
        let output = Command::new("/bin/sh")
            .args(["-c", "\"$1\" \"$2\" >/dev/null 2>&1 & echo $!", "fixture"])
            .arg(&shell)
            .arg(&worker)
            .output()
            .unwrap();
        assert!(output.status.success());
        let pid = String::from_utf8(output.stdout)
            .unwrap()
            .trim()
            .parse::<u32>()
            .unwrap();
        for _ in 0..50 {
            if System::new_all()
                .processes()
                .values()
                .any(|p| p.exe().is_some_and(|exe| exe == &worker))
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let data_path = directory.path().join("state");
        let engine = Engine::new(&data_path).unwrap();
        {
            let mut core = engine.core.lock().unwrap();
            core.roots.clear();
            core.settings.auto_discover = false;
            core.store.set("full_discovery_version", &1u32).unwrap();
        }
        engine.full_scan.store(false, Ordering::Relaxed);
        let candidates = {
            let mut core = engine.core.lock().unwrap();
            let managed = core.collect_processes(&[]);
            process_candidates(&core.system, &managed)
        };
        let candidate = candidates
            .iter()
            .find(|c| c.pid == pid)
            .expect("isolated fixture is visible to its user");
        let cache = directory.path().join("cache");
        fs::create_dir(&cache).unwrap();
        fs::write(cache.join("old.bin"), b"fixture-cache").unwrap();
        let request = AdoptRequest {
            name: "Fixture Agent".into(),
            color: "#528f80".into(),
            pid: Some(pid),
            start_time: Some(candidate.start_time),
            executable: candidate.executable.clone(),
            entrypoint: "".into(),
            remember: true,
            directories: vec![DirectoryBinding {
                path: cache.to_string_lossy().into_owned(),
                usage: "cache".into(),
            }],
        };
        let mut stale = request.clone();
        stale.start_time = Some(candidate.start_time + 1);
        assert!(engine.adopt(stale).is_err());
        let snapshot = engine.adopt(request.clone()).unwrap();
        let definition = snapshot
            .agent_definitions
            .iter()
            .find(|d| !d.builtin && d.enabled)
            .unwrap();
        let id = definition.id.clone();
        let managed = snapshot
            .processes
            .iter()
            .filter(|p| p.agent == id)
            .collect::<Vec<_>>();
        assert!(managed.iter().any(|p| p.pid == pid));
        assert!(
            managed.iter().all(|p| !p.orphan),
            "a live, observed parent chain must not be classified as orphaned"
        );
        let resampled = engine.sample().unwrap();
        assert!(resampled
            .processes
            .iter()
            .filter(|p| p.agent == id)
            .all(|p| !p.orphan));
        assert!(
            request_stop_checked(&engine, pid, candidate.start_time, true)
                .unwrap_err()
                .contains("no longer a cleanup suggestion")
        );
        assert!(System::new_all().process(Pid::from_u32(pid)).is_some());
        assert!(managed
            .iter()
            .any(|p| p.executable == worker.to_string_lossy() && p.evidence.contains("parent")));
        let summary = snapshot.agents.iter().find(|a| a.id == id).unwrap();
        assert_eq!(summary.processes, managed.len());
        assert_eq!(
            summary.memory,
            managed.iter().map(|p| p.memory).sum::<u64>()
        );
        engine.run_inventory(true, &|_| {}).unwrap();
        assert_eq!(
            engine
                .sample()
                .unwrap()
                .agents
                .iter()
                .find(|a| a.id == id)
                .unwrap()
                .storage,
            13
        );
        let restored = Engine::new(&data_path).unwrap();
        assert!(restored.core.lock().unwrap().bindings.is_empty());
        assert!(restored
            .sample()
            .unwrap()
            .processes
            .iter()
            .any(|p| p.pid == pid && p.agent == id));
        let mut duplicate = request.clone();
        duplicate.name = "Duplicate Rule".into();
        duplicate.pid = None;
        duplicate.start_time = None;
        duplicate.directories.clear();
        assert!(engine
            .adopt(duplicate)
            .unwrap_err()
            .contains("already in use"));
        let detached = engine.detach(&id).unwrap();
        assert!(!detached.processes.iter().any(|p| p.agent == id));
        assert!(!detached.agents.iter().any(|a| a.id == id));
        assert!(detached
            .agent_definitions
            .iter()
            .any(|d| d.id == id && !d.enabled));
        assert!(cache.join("old.bin").exists());
        assert!(System::new_all().process(Pid::from_u32(pid)).is_some());
        let store = Store::open(&data_path).unwrap();
        assert_eq!(
            store
                .inventory_details("active", None, "", 0)
                .unwrap()
                .count,
            0
        );
        let mut once = request;
        once.name = "Ephemeral Fixture".into();
        once.remember = false;
        let snapshot = engine.adopt(once).unwrap();
        let ephemeral = snapshot
            .agent_definitions
            .iter()
            .find(|d| d.name == "Ephemeral Fixture")
            .unwrap()
            .id
            .clone();
        let restarted = Engine::new(&data_path).unwrap();
        assert!(!restarted
            .sample()
            .unwrap()
            .processes
            .iter()
            .any(|p| p.agent == ephemeral));
        request_stop(&engine, pid, candidate.start_time).unwrap();
        for _ in 0..40 {
            std::thread::sleep(Duration::from_millis(20));
            let snapshot = engine.sample().unwrap();
            if snapshot
                .processes
                .iter()
                .any(|p| p.agent == ephemeral && p.orphan)
            {
                fs::write(directory.path().join("spawn-grandchild"), b"go").unwrap();
                for _ in 0..50 {
                    std::thread::sleep(Duration::from_millis(20));
                    let snapshot = engine.sample().unwrap();
                    if snapshot
                        .processes
                        .iter()
                        .filter(|p| {
                            p.agent == ephemeral && p.executable == worker.to_string_lossy()
                        })
                        .count()
                        == 2
                    {
                        return;
                    }
                }
                panic!("new descendants of an observed orphan must also remain tracked");
            }
        }
        panic!("a previously observed worker remains attributed when its adopted parent exits");
    }
}
