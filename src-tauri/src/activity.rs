use crate::{collector::now, model::*};
use std::collections::{HashMap, HashSet};
pub fn bucket(kind: &str) -> &str {
    match kind {
        "growth" | "decrease" => "storage",
        other => other,
    }
}
/// Keep infrequent operations visible even when storage emits many observations.
pub fn bound(events: &mut Vec<Activity>) {
    events.sort_by(|a, b| b.time.cmp(&a.time));
    let mut counts: HashMap<String, usize> = HashMap::new();
    events.retain(|event| {
        let n = counts.entry(bucket(&event.kind).into()).or_default();
        *n += 1;
        *n <= 400
    });
}
pub fn process_events(
    previous: &[ProcessEntry],
    current: &[ProcessEntry],
    initialized: bool,
) -> Vec<Activity> {
    let old: HashMap<_, _> = previous
        .iter()
        .map(|p| ((p.pid, p.start_time), p))
        .collect();
    let present: HashSet<_> = current.iter().map(|p| (p.pid, p.start_time)).collect();
    let mut groups: HashMap<(String, &str), Vec<&ProcessEntry>> = HashMap::new();
    for process in current {
        let prior = old.get(&(process.pid, process.start_time));
        if prior.is_none() {
            groups
                .entry((
                    process.agent.clone(),
                    if initialized { "detected" } else { "observed" },
                ))
                .or_default()
                .push(process);
        }
        if process.recommendation == "review" && prior.is_none_or(|p| p.recommendation != "review")
        {
            groups
                .entry((process.agent.clone(), "attention"))
                .or_default()
                .push(process);
        }
    }
    if initialized {
        for process in previous {
            if !present.contains(&(process.pid, process.start_time)) {
                groups
                    .entry((process.agent.clone(), "exited"))
                    .or_default()
                    .push(process);
            }
        }
    }
    let mut events = groups
        .into_iter()
        .map(|((agent, action), processes)| Activity {
            id: uuid::Uuid::new_v4().to_string(),
            time: now(),
            kind: "process".into(),
            agent,
            title: match action {
                "observed" => "Existing Agent processes observed",
                "detected" => "New Agent processes detected",
                "attention" => "Agent processes need attention",
                _ => "Agent processes exited",
            }
            .into(),
            detail: processes
                .iter()
                .take(12)
                .map(|p| format!("{} · PID {}", p.name, p.pid))
                .collect::<Vec<_>>()
                .join("\n"),
            bytes: 0,
            context: Some(ActivityContext {
                action: action.into(),
                count: processes.len(),
                processes: processes.iter().take(12).map(|p| (*p).clone()).collect(),
                ..Default::default()
            }),
        })
        .collect::<Vec<_>>();
    events.sort_by(|a, b| a.agent.cmp(&b.agent).then(a.title.cmp(&b.title)));
    events
}
pub fn discovery_events(roots: &[Root], before: &HashSet<String>, existing: bool) -> Vec<Activity> {
    let mut groups: HashMap<String, Vec<&Root>> = HashMap::new();
    for root in roots
        .iter()
        .filter(|r| r.custom && (existing || !before.contains(&r.path)))
    {
        groups.entry(root.agent.clone()).or_default().push(root);
    }
    groups
        .into_iter()
        .map(|(agent, roots)| Activity {
            id: uuid::Uuid::new_v4().to_string(),
            time: now(),
            kind: "discovery".into(),
            agent,
            title: if existing {
                "Existing monitored projects indexed"
            } else {
                "Projects discovered automatically"
            }
            .into(),
            detail: roots.len().to_string(),
            bytes: 0,
            context: Some(ActivityContext {
                action: if existing { "indexed" } else { "discovered" }.into(),
                count: roots.len(),
                paths: roots.iter().take(20).map(|r| r.path.clone()).collect(),
                ..Default::default()
            }),
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn high_volume_storage_does_not_evict_other_categories() {
        let event = |kind: &str, time| Activity {
            id: format!("{kind}-{time}"),
            time,
            kind: kind.into(),
            agent: "codex".into(),
            title: "".into(),
            detail: "".into(),
            bytes: 0,
            context: None,
        };
        let mut events = (1..=500).map(|i| event("growth", i)).collect::<Vec<_>>();
        events.push(event("cleanup", 0));
        events.push(event("discovery", 0));
        bound(&mut events);
        assert_eq!(events.len(), 402);
        assert!(events.iter().any(|e| e.kind == "cleanup"));
    }
}

#[cfg(test)]
mod observation_tests {
    use super::*;
    use crate::resources::fixture;
    #[test]
    fn process_observations_are_transitions_not_every_sampling_tick() {
        let old = fixture(42, None, 10, "");
        let initial = process_events(&[], &[old.clone()], false);
        assert_eq!(initial.len(), 1);
        assert_eq!(initial[0].context.as_ref().unwrap().action, "observed");
        assert_eq!(initial[0].context.as_ref().unwrap().processes[0].pid, 42);
        assert!(process_events(&[old.clone()], &[old.clone()], true).is_empty());
        let mut attention = old.clone();
        attention.recommendation = "review".into();
        let events = process_events(&[old.clone()], &[attention.clone()], true);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].context.as_ref().unwrap().action, "attention");
        assert!(process_events(&[attention.clone()], &[attention], true).is_empty());
        let reused = fixture(42, None, 20, "");
        let events = process_events(&[old], &[reused], true);
        assert_eq!(events.len(), 2);
        assert!(events
            .iter()
            .any(|e| e.context.as_ref().unwrap().action == "exited"));
        let exited = events
            .iter()
            .find(|e| e.context.as_ref().unwrap().action == "exited")
            .unwrap();
        assert_eq!(exited.context.as_ref().unwrap().processes[0].start_time, 10);
    }
}
