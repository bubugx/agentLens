use crate::model::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcePoint {
    pub time: u64,
    pub memory: u64,
    pub cpu: f32,
    pub processes: usize,
}
pub fn project_key(process: &ProcessEntry) -> String {
    if process.project_path.is_empty() {
        format!("unassigned:{}", process.agent)
    } else {
        process.project_path.clone()
    }
}
pub fn assign_projects(current: &mut [ProcessEntry], previous: &[ProcessEntry], roots: &[Root]) {
    let home = dirs::home_dir().unwrap_or_default();
    for p in current.iter_mut() {
        let cwd = Path::new(&p.working_directory);
        if !p.working_directory.is_empty() {
            if let Some(root) = roots
                .iter()
                .filter(|r| {
                    r.custom
                        && !matches!(
                            r.source.as_str(),
                            "adopted-cache" | "adopted-logs" | "adopted-temporary"
                        )
                        && cwd.starts_with(&r.path)
                })
                .max_by_key(|r| r.path.len())
            {
                p.project_path = root.path.clone();
                p.project_evidence = "monitored_directory".into();
            } else if cwd.starts_with(&home)
                && cwd != home
                && !roots.iter().any(|r| !r.custom && cwd.starts_with(&r.path))
            {
                p.project_path = p.working_directory.clone();
                p.project_evidence = "working_directory".into();
            }
        }
        if p.project_path.is_empty() {
            if let Some(old) = previous.iter().find(|old| {
                old.pid == p.pid && old.start_time == p.start_time && !old.project_path.is_empty()
            }) {
                p.project_path = old.project_path.clone();
                p.project_evidence = "previous_observation".into();
            }
        }
    }
    for _ in 0..64 {
        let parents: HashMap<_, _> = current
            .iter()
            .filter(|p| !p.project_path.is_empty())
            .map(|p| (p.pid, (p.start_time, p.project_path.clone())))
            .collect();
        let mut changed = false;
        for p in current.iter_mut().filter(|p| p.project_path.is_empty()) {
            if let Some((start, path)) = p.parent_pid.and_then(|id| parents.get(&id)) {
                if *start <= p.start_time {
                    p.project_path = path.clone();
                    p.project_evidence = "parent_process".into();
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
}
pub fn aggregate(
    processes: &[ProcessEntry],
    time: u64,
) -> HashMap<(String, String), ResourcePoint> {
    let mut groups = HashMap::new();
    groups.insert(
        ("all".into(), "all".into()),
        ResourcePoint {
            time,
            memory: 0,
            cpu: 0.,
            processes: 0,
        },
    );
    for p in processes {
        for key in [
            ("all".into(), "all".into()),
            ("project".into(), project_key(p)),
            ("process".into(), format!("{}:{}", p.pid, p.start_time)),
        ] {
            let value = groups.entry(key).or_insert(ResourcePoint {
                time,
                memory: 0,
                cpu: 0.,
                processes: 0,
            });
            value.memory += p.memory;
            value.cpu += p.cpu;
            value.processes += 1;
        }
    }
    groups
}

#[cfg(test)]
pub fn fixture(pid: u32, parent: Option<u32>, start: u64, cwd: &str) -> ProcessEntry {
    serde_json::from_value(serde_json::json!({
        "pid": pid,
        "parentPid": parent,
        "startTime": start,
        "name": "fixture",
        "agent": "codex",
        "memory": 100,
        "cpu": 2.5,
        "status": "Run",
        "evidence": "Observed parent process chain",
        "orphan": false,
        "executable": "/fixture/agent",
        "purpose": "agent",
        "purposeEvidence": "fixture",
        "recommendation": "keep",
        "recommendationReason": "fixture",
        "workingDirectory": cwd,
        "runtimeSeconds": 10,
        "memoryDelta": 0,
        "cpuDelta": 0,
        "highCpuSamples": 0,
    }))
    .unwrap()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_attribution_uses_directories_then_verified_parents_and_never_reused_pids() {
        let home = dirs::home_dir().unwrap();
        let project = home.join("agentlens-fixture-project");
        let roots = vec![Root {
            id: "fixture".into(),
            path: project.to_string_lossy().into_owned(),
            agent: "codex".into(),
            label: "Fixture".into(),
            custom: true,
            source: "scan".into(),
        }];
        let mut ps = vec![
            fixture(1, None, 10, &project.join("src").to_string_lossy()),
            fixture(2, Some(1), 11, ""),
            fixture(3, None, 12, ""),
        ];
        assign_projects(&mut ps, &[], &roots);
        assert_eq!(ps[0].project_path, roots[0].path);
        assert_eq!(ps[1].project_path, roots[0].path);
        assert_eq!(ps[1].project_evidence, "parent_process");
        assert!(ps[2].project_path.is_empty());
        let totals = aggregate(&ps, 100);
        assert_eq!(totals[&("all".into(), "all".into())].memory, 300);
        assert_eq!(
            totals[&("project".into(), roots[0].path.clone())].memory,
            200
        );
        assert_eq!(
            totals[&("project".into(), "unassigned:codex".into())].cpu,
            2.5
        );
        let mut reused = vec![fixture(1, None, 99, "")];
        assign_projects(&mut reused, &ps, &roots);
        assert!(reused[0].project_path.is_empty());
        let mut orphan = vec![fixture(2, None, 11, "")];
        assign_projects(&mut orphan, &ps, &roots);
        assert_eq!(orphan[0].project_path, roots[0].path);
    }
}
