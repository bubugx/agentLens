import { AGENT_META, type Activity, type HistoryPoint } from "./types";
import { t } from "./i18n";
export type ActivityFilter = "all" | "storage" | "cleanup" | "process" | "discovery";
export function activityKind(event: Activity): ActivityFilter {
  return ["growth", "decrease"].includes(event.kind)
    ? "storage"
    : ["cleanup", "process", "discovery"].includes(event.kind)
      ? (event.kind as ActivityFilter)
      : "all";
}
export function matchesActivity(event: Activity, filter: ActivityFilter, query: string) {
  return (
    (filter === "all" || activityKind(event) === filter) &&
    `${event.title} ${t(event.title)} ${event.detail} ${event.agent} ${AGENT_META[event.agent]?.name || ""} ${event.context?.paths.join(" ") || ""}`
      .toLowerCase()
      .includes(query.toLowerCase())
  );
}
export function activityCount(event: Activity): number {
  if (event.context) return event.context.count;
  if (event.kind === "discovery") return Number(event.detail) || 0;
  if (event.kind === "cleanup")
    return Number(event.title.match(/^(\d+) (?:files moved|sample files cleaned)/)?.[1]) || 0;
  return event.kind === "process" ? 1 : 0;
}
export function activityAction(event: Activity): string {
  if (event.context) return event.context.action;
  if (event.kind === "process")
    return event.title.startsWith("Termination requested")
      ? "stop_requested"
      : event.title.includes("attention")
        ? "attention"
        : "detected";
  return event.kind === "discovery"
    ? "discovered"
    : event.kind === "cleanup"
      ? "cleanup"
      : event.kind;
}
export function storageSeries(
  history: HistoryPoint[],
  start: number,
  end: number,
  events: Activity[],
) {
  // At most one predecessor establishes the range baseline. Use actual timestamps, never index spacing.
  const ordered = [...history].filter((p) => p.time <= end).sort((a, b) => a.time - b.time);
  const previous = ordered.filter((p) => p.time < start).at(-1);
  const points = ordered.filter((p) => p.time >= start);
  if (previous) points.unshift({ ...previous, time: start });
  if (points.length)
    return {
      mode: "usage" as const,
      points: points.map((p) => ({ time: p.time, value: p.storage })),
    };
  // Historical observations can establish a net-change curve, never a total usage estimate.
  const changes = events
    .filter((e) => activityKind(e) === "storage" && e.time >= start && e.time <= end)
    .sort((a, b) => a.time - b.time);
  let value = 0;
  const fallback = [{ time: start, value }];
  for (const e of changes) {
    value += e.bytes;
    const prior = fallback.at(-1)!;
    if (prior.time === e.time) prior.value = value;
    else fallback.push({ time: e.time, value });
  }
  return { mode: "change" as const, points: changes.length ? fallback : [] };
}
