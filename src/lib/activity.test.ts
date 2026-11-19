import { describe, it, expect } from "vitest";
import { activityCount, matchesActivity, storageSeries } from "./activity";
import { demoSnapshot } from "./demo";
import { projectGroups } from "./resources";
import type { Activity, HistoryPoint } from "./types";
const event = (kind: string, time: number, bytes: number): Activity => ({
  id: `${kind}-${time}`,
  kind,
  time,
  bytes,
  agent: "codex",
  title: "Fixture",
  detail: "",
});
const point = (time: number, storage: number) =>
  ({ time, storage, memory: 0, cpu: 0, agents: [] }) as HistoryPoint;
describe("activity and resource analysis", () => {
  it("storage trends use actual usage samples and a boundary predecessor", () => {
    const result = storageSeries(
      [point(100, 50), point(180, 70), point(300, 60)],
      150,
      300,
      [],
    );
    expect(result.mode).toBe("usage");
    expect(result.points).toEqual([
      { time: 150, value: 50 },
      { time: 180, value: 70 },
      { time: 300, value: 60 },
    ]);
  });
  it("fallback net changes exclude cleanup records to avoid counting the same removal twice", () => {
    const result = storageSeries([], 100, 300, [
      event("growth", 110, 40),
      event("cleanup", 150, -20),
      event("decrease", 180, -20),
    ]);
    expect(result.mode).toBe("change");
    expect(result.points.at(-1)?.value).toBe(20);
  });
  it("category filters and operation counts preserve historical record formats", () => {
    expect(matchesActivity(event("process", 100, 0), "storage", "")).toBe(
      false,
    );
    expect(activityCount({ ...event("discovery", 100, 0), detail: "12" })).toBe(
      12,
    );
    expect(
      activityCount({
        ...event("cleanup", 100, -10),
        title: "3 files moved to Trash",
      }),
    ).toBe(3);
  });
  it("project totals equal the sum of process resources without double counting children", () => {
    const data = demoSnapshot();
    const groups = projectGroups(data.processes, data.roots);
    expect(groups.reduce((s, g) => s + g.memory, 0)).toBe(
      data.processes.reduce((s, p) => s + p.memory, 0),
    );
    expect(groups.reduce((s, g) => s + g.cpu, 0)).toBeCloseTo(
      data.processes.reduce((s, p) => s + p.cpu, 0),
    );
  });
});
