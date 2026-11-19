import { t } from "./i18n";
import type { Process } from "./types";

export const processKey = (process: Process) =>
  `${process.pid}:${process.startTime}`;
export function assessmentCategory(p: Process) {
  return (
    p.assessment?.category ||
    (p.recommendation === "review" || p.purpose === "unknown"
      ? "review"
      : "keep")
  );
}
export function assessmentReason(p: Process) {
  const reason = p.assessment?.reason || "";
  if (reason === "orphan_idle_task")
    return t(
      "Parent chain exited · idle for {0} minutes · no observed disk activity",
      Math.floor((p.assessment?.idleSeconds || 0) / 60),
    );
  return t(
    (
      {
        monitor_ancestor: "AgentLens launch chain · excluded from suggestions",
        resource_activity:
          "CPU, memory or disk activity is still being observed",
        check_task: "Resource or purpose signals need review",
        attached_task: "Connected task or application component",
        service_or_unknown:
          "Service, watch task or uncertain purpose · manual review",
        insufficient_evidence:
          "More continuous observations or readable disk counters are needed",
        observing_idle:
          "Observing whether the orphan stays idle for at least 5 minutes",
        process_state: "Process state requires manual review",
        dependent_process: "A descendant still needs to run or be reviewed",
      } as Record<string, string>
    )[reason] || p.recommendationReason,
  );
}

// Stop selected children before their selected parents so attribution is still
// available when the native command rechecks each process identity.
export function orderProcessStops(processes: Process[]): Process[] {
  const unique = [
    ...new Map(processes.map((p) => [processKey(p), p])).values(),
  ];
  const ordered: Process[] = [];
  const visited = new Set<string>();
  function visit(process: Process) {
    const key = processKey(process);
    if (visited.has(key)) return;
    visited.add(key);
    for (const child of unique)
      if (
        child.parentPid === process.pid &&
        child.startTime >= process.startTime
      )
        visit(child);
    ordered.push(process);
  }
  unique.forEach(visit);
  return ordered;
}
export function purposeLabel(purpose: string) {
  return t(
    (
      {
        agent: "Agent task engine",
        editor: "Editor application",
        renderer: "Editor interface renderer",
        extensions: "Extension host",
        gpu: "GPU rendering helper",
        network: "Network service",
        mcp: "MCP tool server",
        devserver: "Development server",
        build: "Compiler / build task",
        test: "Test runner",
        browser: "Browser / automation helper",
        shell: "Task shell",
        unknown: "Task needs inspection",
      } as Record<string, string>
    )[purpose] || "Task needs inspection",
  );
}
export const recommendationLabel = (value: string) =>
  t(
    value === "review"
      ? "Review task"
      : value === "inspect"
        ? "Inspect purpose"
        : "Keep while in use",
  );
export function stopImpact(purpose: string) {
  return t(
    purpose === "agent"
      ? "Stopping may interrupt the current agent session."
      : ["editor", "renderer", "gpu", "extensions"].includes(purpose)
        ? "Stopping may close or disrupt the editor and its extensions."
        : purpose === "devserver"
          ? "Stopping makes the project's local development service unavailable."
          : purpose === "mcp"
            ? "Stopping disconnects this tool server from its agent."
            : purpose === "build" || purpose === "test"
              ? "Stopping cancels the current build or test task."
              : "Confirm that the related project no longer needs this task before stopping.",
  );
}
