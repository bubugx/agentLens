import { reactive } from "vue";
import { t } from "./i18n";
export type AgentId = string;
export type Page =
  | "Overview"
  | "Agents"
  | "Storage"
  | "Sessions"
  | "Processes"
  | "Cleanup"
  | "Activity"
  | "Settings";
export interface Root {
  id: string;
  path: string;
  agent: string;
  label: string;
  custom: boolean;
  source: string;
}
export interface FileEntry {
  id: string;
  rootId: string;
  path: string;
  relativePath: string;
  agent: string;
  category: string;
  size: number;
  modified: number;
  identity: string;
  eligible: boolean;
  reason: string;
  cleanupStatus: string;
  sessionId: string | null;
}
export interface Agent {
  id: string;
  name: string;
  storage: number;
  memory: number;
  cpu: number;
  files: number;
  sessions: number;
  processes: number;
  reclaimable: number;
  growth: number;
  installed: boolean;
}
export interface Process {
  pid: number;
  parentPid: number | null;
  startTime: number;
  name: string;
  agent: string;
  memory: number;
  cpu: number;
  status: string;
  evidence: string;
  orphan: boolean;
  executable: string;
  purpose: string;
  purposeEvidence: string;
  recommendation: string;
  recommendationReason: string;
  workingDirectory: string;
  projectPath?: string;
  projectEvidence?: string;
  runtimeSeconds: number;
  memoryDelta: number;
  cpuDelta: number;
  highCpuSamples: number;
  observation?: {
    sampledAt: number;
    orphanSince: number;
    idleSince: number;
    idleSamples: number;
    diskRead: number;
    diskWritten: number;
    ioKnown: boolean;
    watchMode: boolean;
  };
  assessment?: {
    category: string;
    reason: string;
    idleSeconds: number;
    orphanSeconds: number;
  };
}
export interface ProcessStopResult {
  requested: Process[];
  failures: { process: Process; reason: string }[];
}
export interface Session {
  id: string;
  agent: string;
  project: string;
  path: string;
  size: number;
  modified: number;
  confidence: string;
}
export interface HistoryPoint {
  time: number;
  storage: number;
  memory: number;
  cpu: number;
  agents: [string, number][];
}
export interface ActivityContext {
  action: string;
  count: number;
  failed: number;
  paths: string[];
  processes?: Process[];
}
export interface Activity {
  id: string;
  time: number;
  kind: string;
  agent: string;
  title: string;
  detail: string;
  bytes: number;
  context?: ActivityContext;
}
export interface Settings {
  language: "zh-CN" | "en-US";
  theme: "light" | "dark" | "custom";
  interval: number;
  retentionDays: number;
  menuBar: boolean;
  customBase: "light" | "dark";
  customAccent: string;
  cleanup: CleanupPolicy;
  autoDiscover: boolean;
}
export interface CleanupPolicy {
  preset: "conservative" | "balanced" | "custom";
  temporaryHours: number;
  cacheDays: number;
  logDays: number;
  includeSessions: boolean;
  sessionDays: number;
  temporaryEnabled: boolean;
  cacheEnabled: boolean;
  logsEnabled: boolean;
  sessionCacheEnabled: boolean;
  sessionCacheDays: number;
  sessionScope: "all" | "archived";
  keepLatestSessions: number;
  includeProjectSessions: boolean;
  minimumSizeMb: number;
  excludedPaths: string[];
  excludedAgents: string[];
}
export interface CleanupBucket {
  code: string;
  count: number;
  bytes: number;
}
export interface ScanProgress {
  background: boolean;
  phase:
    "" | "discovery" | "indexing" | "saving" | "complete" | "paused" | "error";
  initial: boolean;
  directories: number;
  files: number;
  bytes: number;
  projects: number;
  rootsDone: number;
  rootsTotal: number;
  workers: number;
  elapsedMs: number;
  currentPath: string;
  errors: number;
}
export interface InventoryGroup {
  id: string;
  label: string;
  path: string;
  agent: string;
  count: number;
  bytes: number;
  project: boolean;
}
export interface InventoryDetails {
  count: number;
  bytes: number;
  groups: InventoryGroup[];
  files: FileEntry[];
  offset: number;
  limit: number;
}
export interface Snapshot {
  memoryDelta: number;
  cpuDelta: number;
  timestamp: number;
  scannedAt: number;
  monitoring: boolean;
  scanning: boolean;
  platform: string;
  totalMemory: number;
  usedMemory: number;
  diskTotal: number;
  diskAvailable: number;
  agents: Agent[];
  agentDefinitions?: AgentDefinition[];
  roots: Root[];
  files: FileEntry[];
  fileCount: number;
  sessions: Session[];
  processes: Process[];
  history: HistoryPoint[];
  activity: Activity[];
  warnings: string[];
  settings: Settings;
  scanComplete: boolean;
  cleanupSummary: CleanupBucket[];
  scanProgress: ScanProgress;
}
export interface CleanupPlan {
  id: string;
  expiresAt: number;
  items: FileEntry[];
  total: number;
}
export interface ActionResult {
  succeeded: number;
  bytes: number;
  failures: string[];
}
export const AGENT_META: Record<
  string,
  {
    name: string;
    color: string;
    light: string;
    symbol: string;
    enabled?: boolean;
  }
> = reactive({
  unknown: {
    name: "Unassigned projects",
    color: "#8497a7",
    light: "#e7edf2",
    symbol: "◇",
  },
  claude: {
    name: "Claude Code",
    color: "#cd8b70",
    light: "#f4e9e2",
    symbol: "✳",
  },
  codex: { name: "Codex", color: "#528f80", light: "#e3eee9", symbol: "⬡" },
  cursor: { name: "Cursor", color: "#8584b9", light: "#ecebf5", symbol: "➤" },
  gemini: {
    name: "Gemini CLI",
    color: "#7aa4c5",
    light: "#e8eff6",
    symbol: "✦",
  },
  opencode: {
    name: "OpenCode",
    color: "#c6aa6f",
    light: "#f4eedf",
    symbol: "⌘",
  },
  aider: { name: "Aider", color: "#9b9ca1", light: "#ecedee", symbol: "a" },
});
export function bytes(value: number, decimals = 1): string {
  if (!Number.isFinite(value)) return "—";
  if (value === 0) return "0 B";
  const unit = Math.max(
    0,
    Math.min(4, Math.floor(Math.log(Math.abs(value)) / Math.log(1000))),
  );
  return `${(value / 1000 ** unit).toFixed(unit === 0 ? 0 : decimals)} ${["B", "KB", "MB", "GB", "TB"][unit]}`;
}
export function delta(value: number): string {
  return `${value >= 0 ? "+" : "−"}${bytes(Math.abs(value))}`;
}
export function relativeTime(time: number): string {
  const diff = Math.max(0, Math.floor(Date.now() / 1000 - time));
  return diff < 60
    ? t("Just now")
    : diff < 3600
      ? t("{0} min ago", Math.floor(diff / 60))
      : diff < 86400
        ? t("{0} hr ago", Math.floor(diff / 3600))
        : t("{0} days ago", Math.floor(diff / 86400));
}
export const compactPath = (path: string) =>
  path.replace(/^\/Users\/[^/]+|^\/home\/[^/]+|^[A-Za-z]:\\Users\\[^\\]+/, "~");
export interface GrowthSignal {
  metric: "storage" | "memory";
  change: number;
  minutes: number;
}
export function growthSignals(history: HistoryPoint[]): GrowthSignal[] {
  const recent = history.slice(-6);
  if (recent.length < 6) return [];
  const span = recent.at(-1)!.time - recent[0].time;
  if (
    span < 240 ||
    span > 900 ||
    recent.some(
      (p, i) =>
        i > 0 &&
        (p.time - recent[i - 1].time > 150 || p.time <= recent[i - 1].time),
    )
  )
    return [];
  return (["storage", "memory"] as const).flatMap((metric) => {
    const change = recent.at(-1)![metric] - recent[0][metric];
    return change >= (metric === "storage" ? 10e6 : 100e6) &&
      recent.every((p, i) => !i || p[metric] > recent[i - 1][metric])
      ? [{ metric, change, minutes: Math.round(span / 60) }]
      : [];
  });
}

export interface ProcessRule {
  executable: string;
  entrypoint: string;
}
export interface AgentDefinition {
  id: string;
  name: string;
  color: string;
  builtin: boolean;
  enabled: boolean;
  rule: ProcessRule | null;
}
export interface ProcessCandidate {
  pid: number;
  parentPid: number | null;
  startTime: number;
  name: string;
  executable: string;
  workingDirectory: string;
  entrypoint: string;
  memory: number;
  cpu: number;
  agent: string | null;
  canRemember: boolean;
}
export interface DirectoryBinding {
  path: string;
  usage: "project" | "cache" | "logs" | "temporary";
}
export interface AdoptRequest {
  name: string;
  color: string;
  pid: number | null;
  startTime: number | null;
  executable: string;
  entrypoint: string;
  remember: boolean;
  directories: DirectoryBinding[];
}
export interface AdoptionPreview {
  processes: ProcessCandidate[];
  roots: Root[];
  rule: ProcessRule | null;
}
export function syncAgentDefinitions(definitions?: AgentDefinition[]) {
  for (const d of definitions || []) {
    if (d.builtin) continue;
    AGENT_META[d.id] = {
      name: d.name,
      color: d.color,
      light: `${d.color}18`,
      symbol: d.name.slice(0, 1).toUpperCase(),
      enabled: d.enabled,
    };
  }
}
