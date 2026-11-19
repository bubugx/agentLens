import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  demoAdopt,
  demoDetach,
  demoAdoptionPreview,
  restoreDemoAdoption,
  demoProcessCandidates,
} from "./demo-adoption";
import { demoSnapshot } from "./demo";
import { AGENT_META, syncAgentDefinitions, type AdoptRequest } from "./types";
const request = (remember = true): AdoptRequest => ({
  name: "Research",
  color: "#528f80",
  pid: 48120,
  startTime: 1700000100,
  executable: "/usr/local/bin/python3",
  entrypoint: "module:research_agent",
  remember,
  directories: [],
});
beforeEach(() => {
  const store = new Map<string, string>();
  vi.stubGlobal("localStorage", {
    getItem: (key: string) => store.get(key) || null,
    setItem: (key: string, value: string) => store.set(key, value),
  });
});
describe("custom Agent lifecycle", () => {
  it("restores saved recognition, keeps exact script scope, and preserves historical metadata after detach", () => {
    const adopted = demoAdopt(demoSnapshot(), request());
    const id = adopted.agentDefinitions![0].id;
    expect(adopted.processes.filter((p) => p.agent === id)).toHaveLength(2);
    expect(demoProcessCandidates(adopted).find((c) => c.pid === 48130)?.agent).toBeNull();
    const restored = restoreDemoAdoption(demoSnapshot());
    expect(restored.processes.filter((p) => p.agent === id)).toHaveLength(2);
    const detached = demoDetach(restored, id);
    expect(detached.processes.some((p) => p.agent === id)).toBe(false);
    expect(demoProcessCandidates(detached).find((c) => c.pid === 48120)?.agent).toBeNull();
    syncAgentDefinitions(detached.agentDefinitions);
    expect(AGENT_META[id].name).toBe("Research");
    expect(AGENT_META[id].enabled).toBe(false);
  });
  it("rejects stale process identities and directory overlaps before a registry is saved", () => {
    expect(() => demoAdoptionPreview(demoSnapshot(), { ...request(), startTime: 99 })).toThrow(
      /identity changed/,
    );
    expect(() =>
      demoAdoptionPreview(demoSnapshot(), {
        ...request(),
        directories: [{ path: "/Users/alex/.codex", usage: "cache" }],
      }),
    ).toThrow(/overlaps/);
  });
});
