import type { Snapshot } from "./types";
const GB = 1e9,
  MB = 1e6,
  now = Math.floor(Date.now() / 1000);
const agents = [
  {
    id: "claude",
    name: "Claude Code",
    storage: 12.84 * GB,
    memory: 1.24 * GB,
    cpu: 12.4,
    files: 4826,
    sessions: 38,
    processes: 4,
    reclaimable: 2.16 * GB,
    growth: 683 * MB,
    installed: true,
  },
  {
    id: "codex",
    name: "Codex",
    storage: 8.62 * GB,
    memory: 846 * MB,
    cpu: 6.8,
    files: 3102,
    sessions: 24,
    processes: 3,
    reclaimable: 1.42 * GB,
    growth: 392 * MB,
    installed: true,
  },
  {
    id: "cursor",
    name: "Cursor",
    storage: 5.38 * GB,
    memory: 612 * MB,
    cpu: 3.2,
    files: 2674,
    sessions: 16,
    processes: 2,
    reclaimable: 820 * MB,
    growth: 167 * MB,
    installed: true,
  },
  {
    id: "gemini",
    name: "Gemini CLI",
    storage: 1.56 * GB,
    memory: 0,
    cpu: 0,
    files: 894,
    sessions: 8,
    processes: 0,
    reclaimable: 460 * MB,
    growth: 42 * MB,
    installed: true,
  },
  {
    id: "opencode",
    name: "OpenCode",
    storage: 0,
    memory: 0,
    cpu: 0,
    files: 0,
    sessions: 0,
    processes: 0,
    reclaimable: 0,
    growth: 0,
    installed: false,
  },
  {
    id: "aider",
    name: "Aider",
    storage: 0,
    memory: 0,
    cpu: 0,
    files: 0,
    sessions: 0,
    processes: 0,
    reclaimable: 0,
    growth: 0,
    installed: false,
  },
];
export function demoSnapshot(): Snapshot {
  const roots = agents
    .filter((a) => a.installed)
    .map((a) => ({
      id: a.id,
      path: `/Users/alex/.${a.id}`,
      agent: a.id,
      label: `${a.name} storage`,
      custom: false,
      source: "global",
    }));
  const files = [
    ["claude", "cache/old-build-artifacts.tar", "cache", 1.26 * GB, true],
    ["codex", "tmp/previous-run-output.bin", "temporary", 846 * MB, true],
    ["cursor", "logs/renderer-2026-10-28.log", "logs", 820 * MB, true],
    ["claude", "debug/session-debug.log", "logs", 900 * MB, true],
    ["codex", "log/codex-tui.log.1", "logs", 574 * MB, true],
    ["gemini", "cache/expired-context.bin", "cache", 460 * MB, true],
    [
      "claude",
      "projects/agentlens/session-01.jsonl",
      "sessions",
      246 * MB,
      false,
    ],
    ["codex", "sessions/2026/11/rollout-01.jsonl", "sessions", 186 * MB, false],
    [
      "cursor",
      "workspaceStorage/agentlens/state.vscdb",
      "settings",
      94 * MB,
      false,
    ],
    ["claude", "settings.json", "settings", 1520, false],
  ].map(([agent, path, category, size, eligible], i) => ({
    id: `file-${i}`,
    rootId: String(agent),
    path: `/Users/alex/.${agent}/${path}`,
    relativePath: String(path),
    agent: String(agent),
    category: String(category),
    size: Number(size),
    modified: now - (i + 8) * 86400,
    identity: `demo-${i}`,
    eligible: Boolean(eligible),
    reason: eligible
      ? "Inactive sample agent · stable file · outside retention window"
      : "Protected: session, configuration or unclassified data",
    cleanupStatus: eligible
      ? "eligible"
      : category === "sessions"
        ? "sessions"
        : "protected",
    sessionId: null,
  }));
  const processes: Snapshot["processes"] = agents
    .filter((a) => a.processes)
    .flatMap((a) =>
      Array.from({ length: a.processes }, (_, i) => ({
        pid: 34821 + agents.indexOf(a) * 100 + i,
        parentPid: i === 0 ? 1 : 34821 + agents.indexOf(a) * 100,
        startTime: now - 3600,
        name: i === 0 ? a.id : ["node", "python", "esbuild"][i - 1],
        agent: a.id,
        memory: a.memory / a.processes,
        cpu: i === 3 ? 0.1 : a.cpu / a.processes,
        status: "Run",
        evidence:
          i === 0
            ? "Executable name / application path"
            : i === 3
              ? "Previously observed parent chain; parent exited"
              : "Observed parent process chain",
        orphan: i === 3,
        executable: `/usr/local/bin/${i === 0 ? a.id : "node"}`,
        purpose:
          i === 0
            ? "agent"
            : i === 1
              ? "devserver"
              : i === 3
                ? "build"
                : "unknown",
        purposeEvidence:
          i === 0
            ? "Agent command-line executable"
            : i === 3
              ? "Known compiler or build tool entry point"
              : "Known development server entry point",
        recommendation: i === 3 ? "review" : i === 2 ? "inspect" : "keep",
        recommendationReason:
          i === 3
            ? "Observed parent exited; confirm whether this background task is still needed"
            : "Recognized working component; keep it while the related task or application is in use",
        workingDirectory: [
          "/Users/alex/Developer/agentlens",
          "/Users/alex/Developer/portfolio",
          "/Users/alex/Developer/api-service",
        ][agents.indexOf(a) % 3],
        runtimeSeconds: 3600,
        memoryDelta: i === 3 ? 0 : 4e6,
        cpuDelta: 0.2,
        highCpuSamples: 0,
        observation: {
          sampledAt: now,
          orphanSince: i === 3 ? now - 600 : 0,
          idleSince: i === 3 ? now - 600 : 0,
          idleSamples: i === 3 ? 60 : 0,
          diskRead: 1000000,
          diskWritten: 0,
          ioKnown: true,
          watchMode: false,
        },
        assessment: {
          category: i === 3 ? "suggested" : "active",
          reason: i === 3 ? "orphan_idle_task" : "resource_activity",
          idleSeconds: i === 3 ? 600 : 0,
          orphanSeconds: i === 3 ? 600 : 0,
        },
      })),
    );
  return {
    memoryDelta: 64e6,
    cpuDelta: 2.4,
    timestamp: now,
    scannedAt: now,
    monitoring: true,
    scanning: false,
    platform: "preview",
    totalMemory: 32 * GB,
    usedMemory: 19.2 * GB,
    diskTotal: 1e12,
    diskAvailable: 387.4 * GB,
    agents: agents.map((a) => ({
      ...a,
      cpu: processes
        .filter((p) => p.agent === a.id)
        .reduce((s, p) => s + p.cpu, 0),
    })),
    roots,
    files,
    fileCount: 11496,
    sessions: agents
      .filter((a) => a.installed)
      .flatMap((a) =>
        Array.from({ length: a.id === "claude" ? 5 : 3 }, (_, i) => ({
          id: `${a.id}-session-${String(i + 1).padStart(3, "0")}`,
          agent: a.id,
          project: [
            "/Users/alex/Developer/agentlens",
            "/Users/alex/Developer/portfolio",
            "/Users/alex/Developer/api-service",
          ][i % 3],
          path: `/Users/alex/.${a.id}/sessions/session-${i + 1}.jsonl`,
          size: (48 + i * 34) * MB,
          modified: now - (i * 3600 + 120),
          confidence: i % 3 === 2 ? "directory" : "metadata",
        })),
      ),
    processes,
    history: Array.from({ length: 49 }, (_, i) => {
      const t = i / 48;
      const step =
        (Math.sin(i * 0.75) + Math.cos(i * 0.31)) * 0.055 +
        (i > 29 ? 0.26 : i > 18 ? 0.13 : 0);
      const values = [
        6.9 + t * 5.7 + step,
        4.7 + t * 3.8 + step * 0.6,
        3.3 + t * 2 + step * 0.4,
        1.1 + t * 0.46,
      ];
      return {
        time: now - (48 - i) * 1800,
        storage: values.reduce((s, v) => s + v, 0) * GB,
        memory: (1.6 + t * 1.1 + Math.sin(i) * 0.08) * GB,
        cpu: 12 + Math.sin(i * 0.4) * 6,
        agents: ["claude", "codex", "cursor", "gemini"].map(
          (a, j) => [a, values[j] * GB] as [string, number],
        ),
      };
    }),
    activity: [
      {
        id: "discovery-example",
        time: now - 2400,
        kind: "discovery",
        agent: "codex",
        title: "Projects discovered automatically",
        detail: "2",
        bytes: 0,
        context: {
          action: "discovered",
          count: 2,
          failed: 0,
          paths: [
            "/Users/alex/Developer/agentlens",
            "/Users/alex/Developer/api-service",
          ],
        },
      },
      {
        id: "a1",
        time: now - 120,
        kind: "growth",
        agent: "claude",
        title: "New session files created",
        detail: "agentlens · 12 files in project workspace",
        bytes: 124 * MB,
      },
      {
        id: "a2",
        time: now - 840,
        kind: "process",
        agent: "claude",
        title: "A process needs your attention",
        detail: "python · parent agent exited",
        bytes: 0,
        context: {
          action: "attention",
          count: 1,
          failed: 0,
          paths: [],
          processes: [processes.find((p) => p.recommendation === "review")!],
        },
      },
      {
        id: "a3",
        time: now - 1800,
        kind: "cleanup",
        agent: "codex",
        title: "Old temporary files moved to Trash",
        detail: "18 files · recoverable from Trash",
        bytes: -328 * MB,
      },
      {
        id: "a4",
        time: now - 3600,
        kind: "growth",
        agent: "cursor",
        title: "Workspace storage increased",
        detail: "portfolio · workspace cache",
        bytes: 67 * MB,
      },
      {
        id: "a5",
        time: now - 5400,
        kind: "growth",
        agent: "codex",
        title: "Session checkpoint saved",
        detail: "api-service · session metadata",
        bytes: 18 * MB,
      },
    ],
    warnings: [],
    settings: {
      language: "zh-CN",
      theme: "light",
      customBase: "light",
      customAccent: "#167c83",
      autoDiscover: true,
      cleanup: {
        preset: "conservative",
        temporaryHours: 24,
        cacheDays: 7,
        logDays: 7,
        includeSessions: false,
        sessionDays: 30,
        temporaryEnabled: true,
        cacheEnabled: true,
        logsEnabled: true,
        sessionCacheEnabled: false,
        sessionCacheDays: 7,
        sessionScope: "all",
        keepLatestSessions: 10,
        includeProjectSessions: false,
        minimumSizeMb: 0,
        excludedPaths: [],
        excludedAgents: [],
      },
      interval: 5,
      retentionDays: 7,
      menuBar: true,
    },
    cleanupSummary: Array.from(new Set(files.map((f) => f.cleanupStatus))).map(
      (code) => ({
        code,
        count: files.filter((f) => f.cleanupStatus === code).length,
        bytes: files
          .filter((f) => f.cleanupStatus === code)
          .reduce((sum, f) => sum + f.size, 0),
      }),
    ),
    scanComplete: true,
    scanProgress: {
      background: false,
      phase: "complete",
      initial: false,
      directories: 1340,
      files: 11496,
      bytes: 28.4e9,
      projects: 3,
      rootsDone: 4,
      rootsTotal: 4,
      workers: 4,
      elapsedMs: 4320,
      currentPath: "",
      errors: 0,
    },
  };
}
