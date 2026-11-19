import type {
  AdoptRequest,
  AdoptionPreview,
  AgentDefinition,
  Process,
  ProcessCandidate,
  Root,
  Snapshot,
} from "./types";
export const adoptionDemoCandidates: ProcessCandidate[] = [
  {
    pid: 48120,
    parentPid: null,
    startTime: 1700000100,
    name: "python3",
    executable: "/usr/local/bin/python3",
    entrypoint: "module:research_agent",
    workingDirectory: "/Users/alex/Developer/research-agent",
    memory: 384e6,
    cpu: 3.2,
    agent: null,
    canRemember: true,
  },
  {
    pid: 48121,
    parentPid: 48120,
    startTime: 1700000101,
    name: "worker",
    executable: "/Users/alex/Developer/research-agent/worker",
    entrypoint: "",
    workingDirectory: "/Users/alex/Developer/research-agent",
    memory: 64e6,
    cpu: 0.4,
    agent: null,
    canRemember: true,
  },
  {
    pid: 48130,
    parentPid: null,
    startTime: 1700000102,
    name: "node",
    executable: "/usr/local/bin/node",
    entrypoint: "/Users/alex/Developer/automation-agent/index.js",
    workingDirectory: "/Users/alex/Developer/automation-agent",
    memory: 192e6,
    cpu: 1.1,
    agent: null,
    canRemember: true,
  },
];
const storageKey = "agentlens.demo-adoption.v1";
const ephemeral = new Map<number, string>();
export function restoreDemoAdoption(snapshot: Snapshot): Snapshot {
  try {
    const saved = JSON.parse(localStorage.getItem(storageKey) || "null");
    if (saved && Array.isArray(saved.definitions) && Array.isArray(saved.roots)) {
      snapshot = {
        ...snapshot,
        agentDefinitions: saved.definitions,
        roots: [...snapshot.roots, ...saved.roots],
      };
      return reconcileDemoAdoption(snapshot);
    }
  } catch {}
  return snapshot;
}
function persist(snapshot: Snapshot) {
  try {
    localStorage.setItem(
      storageKey,
      JSON.stringify({
        definitions: snapshot.agentDefinitions,
        roots: snapshot.roots.filter((r) => r.source.startsWith("adopted-")),
      }),
    );
  } catch {}
}
export function reconcileDemoAdoption(snapshot: Snapshot): Snapshot {
  const definitions = snapshot.agentDefinitions || [];
  const ownership = new Map<number, string>();
  for (const candidate of adoptionDemoCandidates) {
    const definition = definitions.find(
      (d) =>
        d.enabled &&
        (ephemeral.get(candidate.pid) === d.id ||
          (d.rule?.executable === candidate.executable &&
            d.rule.entrypoint === candidate.entrypoint)),
    );
    if (definition) ownership.set(candidate.pid, definition.id);
  }
  for (const candidate of adoptionDemoCandidates)
    if (!ownership.has(candidate.pid) && candidate.parentPid && ownership.has(candidate.parentPid))
      ownership.set(candidate.pid, ownership.get(candidate.parentPid)!);
  const processes: Process[] = adoptionDemoCandidates
    .filter((c) => ownership.has(c.pid))
    .map((c) => ({
      ...c,
      agent: ownership.get(c.pid)!,
      status: "Running",
      evidence:
        c.parentPid && ownership.get(c.parentPid) === ownership.get(c.pid)
          ? "Observed parent process chain"
          : "User-adopted process identity",
      orphan: false,
      purpose: c.parentPid ? "unknown" : "agent",
      purposeEvidence:
        "Agent ownership explicitly configured by the user; task usefulness still requires review",
      recommendation: "inspect",
      recommendationReason: "Review the related task before stopping",
      runtimeSeconds: 600,
      memoryDelta: 0,
      cpuDelta: 0,
      highCpuSamples: 0,
    }));
  return {
    ...snapshot,
    processes: [...snapshot.processes.filter((p) => !p.agent.startsWith("custom-")), ...processes],
    agents: [
      ...snapshot.agents.filter((a) => !a.id.startsWith("custom-")),
      ...definitions
        .filter((d) => d.enabled)
        .map((d) => {
          const ps = processes.filter((p) => p.agent === d.id);
          return {
            id: d.id,
            name: d.name,
            storage: 0,
            memory: ps.reduce((s, p) => s + p.memory, 0),
            cpu: ps.reduce((s, p) => s + p.cpu, 0),
            files: 0,
            sessions: 0,
            processes: ps.length,
            reclaimable: 0,
            growth: 0,
            installed: true,
          };
        }),
    ],
  };
}
export function demoProcessCandidates(snapshot: Snapshot) {
  return adoptionDemoCandidates.map((c) => ({
    ...c,
    agent: snapshot.processes.find((p) => p.pid === c.pid)?.agent || null,
  }));
}
export function demoAdoptionPreview(snapshot: Snapshot, request: AdoptRequest): AdoptionPreview {
  if (
    !request.name.trim() ||
    request.name.trim().length > 64 ||
    !/^#[\da-f]{6}$/i.test(request.color)
  )
    throw new Error("Enter a name of 1–64 characters and a valid color");
  if (
    [...snapshot.agents, ...(snapshot.agentDefinitions || []).filter((d) => d.enabled)].some(
      (d) => d.name.toLowerCase() === request.name.trim().toLowerCase(),
    )
  )
    throw new Error("An Agent with this name already exists");
  const candidates = demoProcessCandidates(snapshot);
  const root =
    request.pid === null
      ? null
      : candidates.find(
          (c) =>
            c.pid === request.pid &&
            c.startTime === request.startTime &&
            c.executable === request.executable,
        );
  if (request.pid !== null && !root) throw new Error("Process exited or its identity changed");
  if (!root && !request.remember)
    throw new Error("Choose a running process or save a recognition rule");
  if (
    request.remember &&
    (!request.executable.startsWith("/") ||
      (/(node|python[\d.]*)$/.test(request.executable) && !request.entrypoint))
  )
    throw new Error("A shared runtime requires a specific script or module entry point");
  if (root && request.remember && root.entrypoint !== request.entrypoint)
    throw new Error("The rule does not match the selected process entry point");
  const rule = request.remember
    ? { executable: request.executable, entrypoint: request.entrypoint }
    : null;
  if (
    (snapshot.agentDefinitions || []).some(
      (d) => d.enabled && d.rule && JSON.stringify(d.rule) === JSON.stringify(rule),
    )
  )
    throw new Error("This recognition rule is already in use");
  const ids = new Set(
    candidates
      .filter(
        (c) =>
          c.pid === root?.pid ||
          (rule && c.executable === rule.executable && c.entrypoint === rule.entrypoint),
      )
      .map((c) => c.pid),
  );
  for (let i = 0; i < 64; i++)
    for (const c of candidates) if (c.parentPid && ids.has(c.parentPid)) ids.add(c.pid);
  const processes = candidates.filter((c) => ids.has(c.pid));
  if (processes.some((c) => c.agent))
    throw new Error("This process is already managed by an Agent");
  const roots: Root[] = [];
  for (const b of request.directories) {
    if (!/^\/Users\/[^/]+\/.+/.test(b.path) || b.path.split("/").includes(".."))
      throw new Error("Choose a directory inside your home");
    if (
      [...snapshot.roots, ...roots].some(
        (r) =>
          r.path === b.path || r.path.startsWith(b.path + "/") || b.path.startsWith(r.path + "/"),
      )
    )
      throw new Error("This directory overlaps an existing monitoring root");
    roots.push({
      id: crypto.randomUUID(),
      path: b.path,
      agent: "preview",
      label: b.path.split("/").pop() || "Project",
      custom: true,
      source: `adopted-${b.usage}`,
    });
  }
  return { processes, roots, rule };
}
export function demoAdopt(snapshot: Snapshot, request: AdoptRequest): Snapshot {
  const preview = demoAdoptionPreview(snapshot, request);
  const id = `custom-${crypto.randomUUID()}`;
  const definition: AgentDefinition = {
    id,
    name: request.name.trim(),
    color: request.color,
    builtin: false,
    enabled: true,
    rule: preview.rule,
  };
  if (request.pid !== null) ephemeral.set(request.pid, id);
  const next = reconcileDemoAdoption({
    ...snapshot,
    agentDefinitions: [...(snapshot.agentDefinitions || []), definition],
    roots: [...snapshot.roots, ...preview.roots.map((r) => ({ ...r, agent: id }))],
  });
  persist(next);
  return next;
}
export function demoDetach(snapshot: Snapshot, id: string): Snapshot {
  const next = reconcileDemoAdoption({
    ...snapshot,
    agentDefinitions: snapshot.agentDefinitions?.map((d) =>
      d.id === id ? { ...d, enabled: false, rule: null } : d,
    ),
    roots: snapshot.roots.filter((r) => r.agent !== id),
  });
  persist(next);
  return next;
}
