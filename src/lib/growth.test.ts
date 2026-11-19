import { describe, it, expect } from "vitest";
import { growthSignals } from "./types";
const series = (values: number[], gap = 60) =>
  values.map((v, i) => ({
    time: i * gap,
    storage: v * 1e6,
    memory: v * 10e6,
    cpu: 0,
    agents: [] as [string, number][],
  }));
describe("sustained growth observations", () => {
  it("requires enough samples and sufficient observed growth", () => {
    expect(growthSignals(series([1, 2, 3]))).toEqual([]);
    expect(growthSignals(series([1, 2, 3, 4, 5, 6]))).toEqual([]);
  });
  it("reports disk and RSS separately with their actual observation window", () => {
    expect(growthSignals(series([0, 3, 6, 9, 12, 15]))).toEqual([
      { metric: "storage", change: 15e6, minutes: 5 },
      { metric: "memory", change: 150e6, minutes: 5 },
    ]);
  });
  it("rejects a decline, a stale interval and a reversed clock", () => {
    expect(growthSignals(series([1, 5, 4, 9, 12, 19]))).toEqual([]);
    expect(growthSignals(series([0, 3, 6, 9, 12, 15], 300))).toEqual([]);
    const points = series([0, 3, 6, 9, 12, 15]);
    points[3].time = points[2].time - 1;
    expect(growthSignals(points)).toEqual([]);
  });
});
