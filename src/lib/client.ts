import { projectGroups } from "./resources";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  restoreDemoAdoption,
  demoProcessCandidates,
  demoAdoptionPreview,
  demoAdopt,
  demoDetach,
} from "./demo-adoption";
import type { ProcessCandidate, AdoptRequest, AdoptionPreview } from "./types";
import { demoSnapshot } from "./demo";
import { orderProcessStops } from "./processes";
import type {
  ActionResult,
  CleanupPlan,
  Settings,
  Snapshot,
  InventoryDetails,
  ScanProgress,
  Process,
  ProcessStopResult,
} from "./types";
export const native = isTauri();
let preview = native ? demoSnapshot() : restoreDemoAdoption(demoSnapshot());
const callbacks = new Set<(s: Snapshot) => void>();
function emit() {
  preview = { ...preview, timestamp: Math.floor(Date.now() / 1000) };
  callbacks.forEach((cb) => cb(preview));
}
export async function snapshot(): Promise<Snapshot> {
  return native ? invoke("get_snapshot") : preview;
}
export async function subscribe(callback: (s: Snapshot) => void): Promise<() => void> {
  if (native) return listen<Snapshot>("snapshot", (e) => callback(e.payload));
  callbacks.add(callback);
  return () => {
    callbacks.delete(callback);
  };
}
export async function refresh() {
  if (native) return invoke("refresh");
  preview = { ...preview, scannedAt: Math.floor(Date.now() / 1000) };
  emit();
}
export async function refreshProcesses(): Promise<Snapshot> {
  if (native) return invoke("refresh_process_analysis");
  emit();
  return preview;
}
export async function monitoring(enabled: boolean) {
  if (native) return invoke("set_monitoring", { enabled });
  preview = { ...preview, monitoring: enabled };
  emit();
}
export async function saveSettings(settings: Settings) {
  if (native) return invoke("save_settings", { settings });
  preview = { ...preview, settings };
  emit();
}
export async function addRoot(path: string, agent: string) {
  if (native) return invoke("add_root", { path, agent });
  if (preview.roots.some((r) => r.path === path))
    throw new Error("This directory is already monitored.");
  preview = {
    ...preview,
    roots: [
      ...preview.roots,
      {
        id: crypto.randomUUID(),
        path,
        agent,
        label: path.split("/").pop() || "Project",
        custom: true,
        source: "manual",
      },
    ],
  };
  emit();
}
export async function removeRoot(id: string) {
  if (native) return invoke("remove_root", { id });
  preview = { ...preview, roots: preview.roots.filter((r) => r.id !== id) };
  emit();
}
export async function prepareCleanup(ids: string[]): Promise<CleanupPlan> {
  if (native) return invoke("prepare_cleanup", { ids });
  const items = preview.files.filter((f) => ids.includes(f.id) && f.eligible);
  return {
    id: crypto.randomUUID(),
    expiresAt: Math.floor(Date.now() / 1000) + 60,
    items,
    total: items.reduce((s, f) => s + f.size, 0),
  };
}
export async function executeCleanup(plan: CleanupPlan): Promise<ActionResult> {
  if (native) return invoke("execute_cleanup", { planId: plan.id });
  if (plan.expiresAt < Date.now() / 1000)
    throw new Error("Preview plan expired. Review files again.");
  const ids = new Set(plan.items.map((i) => i.id));
  preview = {
    ...preview,
    files: preview.files.filter((f) => !ids.has(f.id)),
    agents: preview.agents.map((a) => {
      const removed = plan.items.filter((f) => f.agent === a.id);
      const amount = removed.reduce((s, f) => s + f.size, 0);
      return {
        ...a,
        storage: Math.max(0, a.storage - amount),
        reclaimable: Math.max(0, a.reclaimable - amount),
        files: a.files - removed.length,
      };
    }),
    activity: [
      {
        id: crypto.randomUUID(),
        time: Math.floor(Date.now() / 1000),
        kind: "cleanup",
        agent: "system",
        title: `${plan.items.length} sample files cleaned up`,
        detail: "Preview only · no real files were changed",
        bytes: -plan.total,
        context: {
          action: "cleanup",
          count: plan.items.length,
          failed: 0,
          paths: [],
        },
      },
      ...preview.activity,
    ],
  };
  emit();
  return { succeeded: plan.items.length, bytes: plan.total, failures: [] };
}
export async function stopProcess(pid: number, startTime: number) {
  if (native) return invoke("stop_process", { pid, startTime });
  if (!preview.processes.some((p) => p.pid === pid && p.startTime === startTime))
    throw new Error("Process exited or its identity changed");
  preview = {
    ...preview,
    processes: preview.processes.filter((p) => p.pid !== pid),
    activity: [
      {
        id: crypto.randomUUID(),
        time: Math.floor(Date.now() / 1000),
        kind: "process",
        agent: preview.processes.find((p) => p.pid === pid)!.agent,
        title: `Termination requested for ${preview.processes.find((p) => p.pid === pid)!.name}`,
        detail: `PID ${pid} · Preview only`,
        bytes: 0,
        context: { action: "stop_requested", count: 1, failed: 0, paths: [] },
      },
      ...preview.activity,
    ],
  };
  preview.agents = preview.agents.map((a) => ({
    ...a,
    processes: preview.processes.filter((p) => p.agent === a.id).length,
    memory: preview.processes.filter((p) => p.agent === a.id).reduce((s, p) => s + p.memory, 0),
    cpu: preview.processes.filter((p) => p.agent === a.id).reduce((s, p) => s + p.cpu, 0),
  }));
  emit();
}
export async function stopProcesses(
  processes: Process[],
  suggestedOnly = false,
): Promise<ProcessStopResult> {
  const result: ProcessStopResult = { requested: [], failures: [] };
  for (const process of orderProcessStops(processes)) {
    try {
      // Reuse the native PID + start-time and ownership checks for every item.
      if (suggestedOnly && native)
        await invoke("stop_suggested_process", {
          pid: process.pid,
          startTime: process.startTime,
        });
      else {
        if (
          suggestedOnly &&
          preview.processes.find((p) => p.pid === process.pid && p.startTime === process.startTime)
            ?.assessment?.category !== "suggested"
        )
          throw new Error("Process activity changed; this task is no longer a cleanup suggestion");
        await stopProcess(process.pid, process.startTime);
      }
      result.requested.push(process);
    } catch (error) {
      result.failures.push({
        process,
        reason: error instanceof Error ? error.message : String(error),
      });
    }
  }
  return result;
}
export async function reveal(path: string) {
  if (native) {
    const { revealItemInDir } = await import("@tauri-apps/plugin-opener");
    await revealItemInDir(path);
  } else {
    await navigator.clipboard.writeText(path);
  }
}

export async function subscribeProgress(callback: (p: ScanProgress) => void): Promise<() => void> {
  return native ? listen<ScanProgress>("scan-progress", (e) => callback(e.payload)) : () => {};
}
export async function fullRescan() {
  if (native) return invoke("full_rescan");
  if (preview.scanning) return;
  preview = {
    ...preview,
    scanning: true,
    scannedAt: 0,
    scanProgress: {
      ...preview.scanProgress,
      initial: true,
      background: false,
      phase: "discovery",
      directories: 0,
      files: 0,
      bytes: 0,
      rootsDone: 0,
      elapsedMs: 0,
    },
  };
  emit();
  for (let step = 1; step <= 8; step++) {
    await new Promise((resolve) => setTimeout(resolve, 350));
    while (!preview.monitoring) {
      preview = {
        ...preview,
        scanning: false,
        scanProgress: { ...preview.scanProgress, phase: "paused" },
      };
      emit();
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    preview = { ...preview, scanning: true };
    preview = {
      ...preview,
      scanProgress: {
        ...preview.scanProgress,
        phase: step < 3 ? "discovery" : step < 7 ? "indexing" : step === 7 ? "saving" : "complete",
        directories: step * 167,
        files: step * 1437,
        bytes: step * 3.55e9,
        rootsDone: Math.max(0, step - 4),
        elapsedMs: step * 350,
        currentPath: "/Users/alex/Developer/agentlens",
      },
    };
    if (step === 8)
      preview = {
        ...preview,
        scanning: false,
        scannedAt: Math.floor(Date.now() / 1000),
      };
    emit();
  }
}
export async function inventoryDetails(
  status: string,
  root: string | null,
  query: string,
  offset = 0,
): Promise<InventoryDetails> {
  if (native) return invoke("inventory_details", { status, root, query, offset });
  const matches = preview.files.filter(
    (f) => f.cleanupStatus === status && f.path.toLowerCase().includes(query.toLowerCase()),
  );
  const groups = preview.roots
    .map((r) => {
      const files = matches.filter((f) => f.rootId === r.id);
      return {
        id: r.id,
        label: r.label,
        path: r.path,
        agent: r.agent,
        project: r.custom,
        count: files.length,
        bytes: files.reduce((sum, f) => sum + f.size, 0),
      };
    })
    .filter((g) => g.count)
    .sort((a, b) => b.bytes - a.bytes);
  const files = matches
    .filter((f) => !root || f.rootId === root)
    .sort((a, b) => b.size - a.size || a.path.localeCompare(b.path));
  return {
    groups,
    files: files.slice(offset, offset + 50),
    count: files.length,
    bytes: files.reduce((sum, f) => sum + f.size, 0),
    offset,
    limit: 50,
  };
}

export async function listProcessCandidates(): Promise<ProcessCandidate[]> {
  return native ? invoke("list_process_candidates") : demoProcessCandidates(preview);
}
export async function previewAdoption(request: AdoptRequest): Promise<AdoptionPreview> {
  return native ? invoke("preview_adoption", { request }) : demoAdoptionPreview(preview, request);
}
export async function adoptAgent(request: AdoptRequest): Promise<Snapshot> {
  if (native) return invoke("adopt_agent", { request });
  preview = demoAdopt(preview, request);
  emit();
  return preview;
}
export async function detachAgent(id: string): Promise<Snapshot> {
  if (native) return invoke("detach_agent", { id });
  preview = demoDetach(preview, id);
  emit();
  return preview;
}

export async function resourceHistory(
  scope: "all" | "project" | "process",
  id: string,
  seconds: number,
): Promise<import("./resources").ResourcePoint[]> {
  if (native) return invoke("resource_history", { scope, id, seconds });
  const processes =
    scope === "all"
      ? preview.processes
      : scope === "project"
        ? projectGroups(preview.processes, preview.roots).find((p) => p.id === id)?.processes || []
        : preview.processes.filter((p) => `${p.pid}:${p.startTime}` === id);
  const memory = processes.reduce((s, p) => s + p.memory, 0),
    cpu = processes.reduce((s, p) => s + p.cpu, 0),
    end = Math.floor(Date.now() / 1000);
  return Array.from({ length: 60 }, (_, i) => ({
    time: end - seconds + (i / 59) * seconds,
    memory: Math.round(memory * (0.7 + (0.3 * i) / 59 + 0.03 * Math.sin(i * 0.6))),
    cpu: Math.max(0, cpu * (0.6 + (0.4 * i) / 59 + 0.2 * Math.sin(i * 0.4))),
    processes: processes.length,
  }));
}
