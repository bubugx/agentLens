use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub id: String,
    pub path: String,
    pub agent: String,
    pub label: String,
    pub custom: bool,
    #[serde(default)]
    pub source: String,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub id: String,
    pub root_id: String,
    pub path: String,
    pub relative_path: String,
    pub agent: String,
    pub category: String,
    pub size: u64,
    pub modified: u64,
    pub identity: String,
    pub eligible: bool,
    pub reason: String,
    pub cleanup_status: String,
    pub session_id: Option<String>,
}
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AgentSummary {
    pub id: String,
    pub name: String,
    pub storage: u64,
    pub memory: u64,
    pub cpu: f32,
    pub files: usize,
    pub sessions: usize,
    pub processes: usize,
    pub reclaimable: u64,
    pub growth: i64,
    pub installed: bool,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProcessEntry {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub start_time: u64,
    pub name: String,
    pub agent: String,
    pub memory: u64,
    pub cpu: f32,
    pub status: String,
    pub evidence: String,
    pub orphan: bool,
    pub executable: String,
    pub purpose: String,
    pub purpose_evidence: String,
    pub recommendation: String,
    pub recommendation_reason: String,
    pub working_directory: String,
    #[serde(default)]
    pub project_path: String,
    #[serde(default)]
    pub project_evidence: String,
    pub runtime_seconds: u64,
    pub memory_delta: i64,
    pub cpu_delta: f32,
    pub high_cpu_samples: u32,
    #[serde(default)]
    pub observation: ProcessObservation,
    #[serde(default)]
    pub assessment: ProcessAssessment,
}
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProcessObservation {
    pub sampled_at: u64,
    pub orphan_since: u64,
    pub idle_since: u64,
    pub idle_samples: u32,
    pub disk_read: u64,
    pub disk_written: u64,
    pub io_known: bool,
    pub watch_mode: bool,
}
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ProcessAssessment {
    pub category: String,
    pub reason: String,
    pub idle_seconds: u64,
    pub orphan_seconds: u64,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub agent: String,
    pub project: String,
    pub path: String,
    pub size: u64,
    pub modified: u64,
    pub confidence: String,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPoint {
    pub time: u64,
    pub storage: u64,
    pub memory: u64,
    pub cpu: f32,
    pub agents: Vec<(String, u64)>,
}
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ActivityContext {
    pub action: String,
    pub count: usize,
    pub failed: usize,
    pub paths: Vec<String>,
    #[serde(default)]
    pub processes: Vec<ProcessEntry>,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub id: String,
    pub time: u64,
    pub kind: String,
    pub agent: String,
    pub title: String,
    pub detail: String,
    pub bytes: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<ActivityContext>,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Settings {
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    pub interval: u64,
    pub retention_days: u64,
    pub menu_bar: bool,
    #[serde(default = "default_custom_base")]
    pub custom_base: String,
    #[serde(default = "default_custom_accent")]
    pub custom_accent: String,
    pub cleanup: CleanupPolicy,
    #[serde(default = "default_auto_discover")]
    pub auto_discover: bool,
}
fn default_language() -> String {
    "zh-CN".into()
}
fn default_theme() -> String {
    "light".into()
}
fn default_custom_base() -> String {
    "light".into()
}
fn default_custom_accent() -> String {
    "#167c83".into()
}
fn default_auto_discover() -> bool {
    true
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct CleanupPolicy {
    pub preset: String,
    pub temporary_hours: u64,
    pub cache_days: u64,
    pub log_days: u64,
    pub include_sessions: bool,
    pub session_days: u64,
    pub temporary_enabled: bool,
    pub cache_enabled: bool,
    pub logs_enabled: bool,
    pub session_cache_enabled: bool,
    pub session_cache_days: u64,
    pub session_scope: String,
    pub keep_latest_sessions: usize,
    pub include_project_sessions: bool,
    pub minimum_size_mb: u64,
    pub excluded_paths: Vec<String>,
    pub excluded_agents: Vec<String>,
}
impl Default for CleanupPolicy {
    fn default() -> Self {
        Self {
            preset: "conservative".into(),
            temporary_hours: 24,
            cache_days: 7,
            log_days: 7,
            include_sessions: false,
            session_days: 30,
            temporary_enabled: true,
            cache_enabled: true,
            logs_enabled: true,
            session_cache_enabled: false,
            session_cache_days: 7,
            session_scope: "all".into(),
            keep_latest_sessions: 10,
            include_project_sessions: false,
            minimum_size_mb: 0,
            excluded_paths: vec![],
            excluded_agents: vec![],
        }
    }
}
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct CleanupBucket {
    pub code: String,
    pub count: usize,
    pub bytes: u64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            language: "zh-CN".into(),
            theme: "light".into(),
            interval: 5,
            retention_days: 7,
            menu_bar: true,
            custom_base: "light".into(),
            custom_accent: "#167c83".into(),
            cleanup: CleanupPolicy::default(),
            auto_discover: true,
        }
    }
}
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub phase: String,
    pub background: bool,
    pub initial: bool,
    pub directories: u64,
    pub files: u64,
    pub bytes: u64,
    pub projects: usize,
    pub roots_done: usize,
    pub roots_total: usize,
    pub workers: usize,
    pub elapsed_ms: u64,
    pub current_path: String,
    pub errors: u64,
}
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InventoryGroup {
    pub id: String,
    pub label: String,
    pub path: String,
    pub agent: String,
    pub count: usize,
    pub bytes: u64,
    pub project: bool,
}
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InventoryDetails {
    pub count: usize,
    pub bytes: u64,
    pub groups: Vec<InventoryGroup>,
    pub files: Vec<FileEntry>,
    pub offset: usize,
    pub limit: usize,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub memory_delta: i64,
    pub cpu_delta: f32,
    pub timestamp: u64,
    pub scanned_at: u64,
    pub monitoring: bool,
    pub scanning: bool,
    pub platform: String,
    pub total_memory: u64,
    pub used_memory: u64,
    pub disk_total: u64,
    pub disk_available: u64,
    pub agents: Vec<AgentSummary>,
    pub agent_definitions: Vec<crate::adoption::AgentDefinition>,
    pub roots: Vec<Root>,
    pub files: Vec<FileEntry>,
    pub file_count: usize,
    pub sessions: Vec<Session>,
    pub processes: Vec<ProcessEntry>,
    pub history: Vec<HistoryPoint>,
    pub activity: Vec<Activity>,
    pub warnings: Vec<String>,
    pub settings: Settings,
    pub scan_complete: bool,
    pub cleanup_summary: Vec<CleanupBucket>,
    pub scan_progress: ScanProgress,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CleanupPlan {
    pub id: String,
    pub expires_at: u64,
    pub items: Vec<FileEntry>,
    pub total: u64,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ActionResult {
    pub succeeded: usize,
    pub bytes: u64,
    pub failures: Vec<String>,
}
