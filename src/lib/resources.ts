import type { Process, Root } from "./types";
export interface ResourcePoint {
  time: number;
  memory: number;
  cpu: number;
  processes: number;
}
export function projectFor(
  p: Process,
  processes: Process[],
  roots: Root[],
): { path: string; evidence: string } {
  if (p.projectPath)
    return {
      path: p.projectPath,
      evidence: p.projectEvidence || "working_directory",
    };
  if (p.projectEvidence === "unassigned") return { path: "", evidence: "unassigned" };
  const seen = new Set<number>();
  let current: Process | undefined = p;
  while (current && !seen.has(current.pid)) {
    seen.add(current.pid);
    if (current.projectPath) return { path: current.projectPath, evidence: "parent_process" };
    const cwd = current.workingDirectory;
    if (
      cwd &&
      cwd !== "/" &&
      !/^[a-z]:[\\/]?$/i.test(cwd) &&
      !/^\/(Users|home)\/[^/]+\/?$/.test(cwd)
    ) {
      const root = roots
        .filter(
          (r) =>
            r.custom &&
            !["adopted-cache", "adopted-logs", "adopted-temporary"].includes(r.source) &&
            (cwd === r.path || cwd.startsWith(r.path + "/")),
        )
        .sort((a, b) => b.path.length - a.path.length)[0];
      if (root)
        return {
          path: root.path,
          evidence: current === p ? "monitored_directory" : "parent_process",
        };
      if (!roots.some((r) => !r.custom && (cwd === r.path || cwd.startsWith(r.path + "/"))))
        return {
          path: cwd,
          evidence: current === p ? "working_directory" : "parent_process",
        };
    }
    const parent = processes.find(
      (other) => other.pid === current!.parentPid && other.startTime <= current!.startTime,
    );
    current = parent;
  }
  return { path: "", evidence: "unassigned" };
}
export function projectGroups(processes: Process[], roots: Root[]) {
  const map = new Map<
    string,
    {
      id: string;
      path: string;
      agent: string;
      processes: Process[];
      memory: number;
      cpu: number;
      memoryDelta: number;
      cpuDelta: number;
    }
  >();
  for (const p of processes) {
    const path = projectFor(p, processes, roots).path,
      id = path || `unassigned:${p.agent}`;
    const group = map.get(id) || {
      id,
      path,
      agent: p.agent,
      processes: [],
      memory: 0,
      cpu: 0,
      memoryDelta: 0,
      cpuDelta: 0,
    };
    group.processes.push(p);
    group.memory += p.memory;
    group.cpu += p.cpu;
    group.memoryDelta += p.memoryDelta;
    group.cpuDelta += p.cpuDelta;
    map.set(id, group);
  }
  return [...map.values()];
}
