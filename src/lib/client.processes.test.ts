import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Process } from "./types";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke }));
import { stopProcesses } from "./client";
import { orderProcessStops } from "./processes";

const process = (pid: number, parentPid: number | null, startTime = pid) =>
  ({ pid, parentPid, startTime, name: `process-${pid}` }) as Process;

describe("batch process termination", () => {
  beforeEach(() => invoke.mockReset());

  it("deduplicates exact identities and sends selected descendants before ancestors", async () => {
    invoke.mockResolvedValue(undefined);
    const parent = process(10, null),
      child = process(11, 10),
      grandchild = process(12, 11);
    const result = await stopProcesses([parent, child, grandchild, child]);
    expect(invoke.mock.calls).toEqual([
      ["stop_process", { pid: 12, startTime: 12 }],
      ["stop_process", { pid: 11, startTime: 11 }],
      ["stop_process", { pid: 10, startTime: 10 }],
    ]);
    expect(result.requested).toEqual([grandchild, child, parent]);
    expect(result.failures).toEqual([]);
  });

  it("reports rejected identities and protected processes while continuing other requests", async () => {
    invoke
      .mockRejectedValueOnce("Process exited or its identity changed")
      .mockRejectedValueOnce(
        "Cannot stop AgentLens or a process that launched it",
      )
      .mockResolvedValueOnce(undefined);
    const stale = process(20, null, 100),
      protectedProcess = process(21, null),
      valid = process(22, null);
    const result = await stopProcesses([stale, protectedProcess, valid]);
    expect(result.requested).toEqual([valid]);
    expect(result.failures).toEqual([
      { process: stale, reason: "Process exited or its identity changed" },
      {
        process: protectedProcess,
        reason: "Cannot stop AgentLens or a process that launched it",
      },
    ]);
    expect(invoke).toHaveBeenCalledTimes(3);
    expect(invoke.mock.calls[0][1]).toEqual({ pid: 20, startTime: 100 });
  });

  it("keeps a reused PID distinct and tolerates cyclic parent metadata", () => {
    const old = process(30, 31, 100),
      reused = process(30, null, 200),
      other = process(31, 30, 100);
    const ordered = orderProcessStops([old, reused, other]);
    expect(ordered).toHaveLength(3);
    expect(new Set(ordered.map((p) => `${p.pid}:${p.startTime}`)).size).toBe(3);
  });
  it("rechecks automatic suggestions using the guarded command and never falls back to an unguarded stop", async () => {
    invoke
      .mockRejectedValueOnce(
        "Process activity changed; this task is no longer a cleanup suggestion",
      )
      .mockResolvedValueOnce(undefined);
    const stale = process(40, null),
      valid = process(41, null);
    const result = await stopProcesses([stale, valid], true);
    expect(invoke.mock.calls).toEqual([
      ["stop_suggested_process", { pid: 40, startTime: 40 }],
      ["stop_suggested_process", { pid: 41, startTime: 41 }],
    ]);
    expect(result.requested).toEqual([valid]);
    expect(result.failures).toHaveLength(1);
  });
});
