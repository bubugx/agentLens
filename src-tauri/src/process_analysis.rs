use crate::model::*;
use std::collections::{HashMap, HashSet};

const QUIET_SECONDS: u64 = 300;

// A zero counter is valid evidence only when the OS call succeeded.
pub fn disk_counters(pid: u32) -> Option<(u64, u64)> {
    #[cfg(target_os = "macos")]
    {
        let mut usage = std::mem::MaybeUninit::<libc::rusage_info_v2>::uninit();
        let result = unsafe {
            libc::proc_pid_rusage(pid as _, libc::RUSAGE_INFO_V2, usage.as_mut_ptr() as _)
        };
        if result != 0 {
            return None;
        }
        let usage = unsafe { usage.assume_init() };
        Some((usage.ri_diskio_bytesread, usage.ri_diskio_byteswritten))
    }
    #[cfg(target_os = "linux")]
    {
        let counters = std::fs::read_to_string(format!("/proc/{pid}/io")).ok()?;
        let value = |key: &str| {
            counters
                .lines()
                .find_map(|line| line.strip_prefix(key)?.trim().parse::<u64>().ok())
        };
        Some((value("read_bytes:")?, value("write_bytes:")?))
    }
    #[cfg(target_os = "windows")]
    {
        #[repr(C)]
        #[derive(Default)]
        struct IoCounters {
            read_ops: u64,
            write_ops: u64,
            other_ops: u64,
            read_bytes: u64,
            write_bytes: u64,
            other_bytes: u64,
        }
        #[link(name = "kernel32")]
        extern "system" {
            fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut std::ffi::c_void;
            fn GetProcessIoCounters(
                handle: *mut std::ffi::c_void,
                counters: *mut IoCounters,
            ) -> i32;
            fn CloseHandle(handle: *mut std::ffi::c_void) -> i32;
        }
        let mut counters = IoCounters::default();
        unsafe {
            let handle = OpenProcess(0x1000, 0, pid);
            if handle.is_null() {
                return None;
            }
            let ok = GetProcessIoCounters(handle, &mut counters) != 0;
            CloseHandle(handle);
            ok.then_some((counters.read_bytes, counters.write_bytes))
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = pid;
        None
    }
}

pub fn observe(p: &mut ProcessEntry, prior: Option<&ProcessEntry>, time: u64) {
    let old = prior.filter(|old| old.pid == p.pid && old.start_time == p.start_time);
    let continuous = old.is_some_and(|old| {
        old.observation.sampled_at > 0
            && time >= old.observation.sampled_at
            && time - old.observation.sampled_at <= 60
    });
    p.observation.sampled_at = time;
    p.observation.orphan_since = if p.orphan {
        old.filter(|old| continuous && old.orphan && old.observation.orphan_since > 0)
            .map_or(time, |old| old.observation.orphan_since)
    } else {
        0
    };
    p.observation.io_known = old.is_some_and(|old| {
        continuous
            && old.observation.io_readable
            && p.observation.io_readable
            && p.observation.disk_read >= old.observation.disk_read
            && p.observation.disk_written >= old.observation.disk_written
    });
    p.observation.idle_since = 0;
    p.observation.idle_samples = 0;
    let quiet = p.cpu < 1.0
        && p.cpu.is_finite()
        && p.memory_delta.unsigned_abs() <= 1_000_000
        && old.is_some_and(|old| {
            continuous
                && p.observation.io_known
                && p.observation.disk_read == old.observation.disk_read
                && p.observation.disk_written == old.observation.disk_written
        });
    if quiet {
        let old = old.unwrap();
        p.observation.idle_since = if old.observation.idle_since > 0 {
            old.observation.idle_since
        } else {
            time
        };
        p.observation.idle_samples = old.observation.idle_samples.saturating_add(u32::from(
            time.saturating_sub(old.observation.sampled_at) >= 2,
        ));
    }
}

pub fn assess(processes: &mut [ProcessEntry], protected: &HashSet<u32>, time: u64) {
    for p in processes.iter_mut() {
        let idle = if p.observation.idle_since > 0 {
            time.saturating_sub(p.observation.idle_since)
        } else {
            0
        };
        let orphan = if p.observation.orphan_since > 0 {
            time.saturating_sub(p.observation.orphan_since)
        } else {
            0
        };
        let (category, reason) = if protected.contains(&p.pid) {
            ("protected", "monitor_ancestor")
        } else if p.cpu >= 1.0
            || p.memory_delta.unsigned_abs() > 1_000_000
            || (p.observation.idle_since == 0 && p.observation.io_known)
        {
            ("active", "resource_activity")
        } else if !p.orphan {
            if p.purpose == "unknown" || p.high_cpu_samples >= 3 || p.memory >= 1_500_000_000 {
                ("review", "check_task")
            } else {
                ("keep", "attached_task")
            }
        } else if p.observation.watch_mode
            || matches!(
                p.purpose.as_str(),
                "agent"
                    | "editor"
                    | "renderer"
                    | "extensions"
                    | "gpu"
                    | "network"
                    | "mcp"
                    | "devserver"
                    | "browser"
            )
        {
            ("keep", "persistent_service")
        } else if !matches!(p.purpose.as_str(), "build" | "test") {
            ("review", "service_or_unknown")
        } else if !p.observation.io_known
            || p.observation.sampled_at == 0
            || time.saturating_sub(p.observation.sampled_at) > 60
        {
            ("observing", "insufficient_evidence")
        } else if orphan < QUIET_SECONDS || idle < QUIET_SECONDS || p.observation.idle_samples < 10
        {
            ("observing", "observing_idle")
        } else if !matches!(p.status.as_str(), "Run" | "Sleep" | "Idle") {
            ("review", "process_state")
        } else {
            ("suggested", "orphan_idle_task")
        };
        p.assessment = ProcessAssessment {
            category: category.into(),
            reason: reason.into(),
            idle_seconds: idle,
            orphan_seconds: orphan,
        };
    }
    // A quiet parent can be waiting for a working child. Keep the whole branch
    // unless every observed descendant is independently eligible.
    for _ in 0..64 {
        let mut blocked: HashMap<u32, u64> = HashMap::new();
        for child in processes
            .iter()
            .filter(|p| p.assessment.category != "suggested")
        {
            if let Some(parent) = child.parent_pid {
                blocked
                    .entry(parent)
                    .and_modify(|start| *start = (*start).max(child.start_time))
                    .or_insert(child.start_time);
            }
        }
        let mut changed = false;
        for p in processes
            .iter_mut()
            .filter(|p| p.assessment.category == "suggested")
        {
            if blocked
                .get(&p.pid)
                .is_some_and(|start| *start >= p.start_time)
            {
                p.assessment.category = "review".into();
                p.assessment.reason = "dependent_process".into();
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::fixture;
    fn candidate(pid: u32, parent: Option<u32>) -> ProcessEntry {
        let mut p = fixture(pid, parent, 10, "");
        p.orphan = true;
        p.cpu = 0.1;
        p.purpose = "build".into();
        p.status = "Sleep".into();
        p.observation = ProcessObservation {
            sampled_at: 1000,
            orphan_since: 600,
            idle_since: 600,
            idle_samples: 40,
            disk_read: 100,
            io_known: true,
            io_readable: true,
            ..Default::default()
        };
        p
    }
    #[test]
    fn only_observed_idle_orphan_tasks_are_suggested_and_children_block_parents() {
        let p = candidate(1, None);
        let mut active = candidate(2, None);
        active.cpu = 97.;
        let mut service = candidate(3, None);
        service.purpose = "mcp".into();
        let mut watch = candidate(4, None);
        watch.observation.watch_mode = true;
        let mut unknown_io = candidate(5, None);
        unknown_io.observation.io_known = false;
        let mut recent = candidate(6, None);
        recent.observation.orphan_since = 990;
        let mut attached = candidate(7, None);
        attached.orphan = false;
        let parent = candidate(8, None);
        let mut child = candidate(9, Some(8));
        child.cpu = 50.;
        let protected = candidate(10, None);
        let mut ps = vec![
            p, active, service, watch, unknown_io, recent, attached, parent, child, protected,
        ];
        assess(&mut ps, &HashSet::from([10]), 1000);
        assert_eq!(
            ps.iter()
                .filter(|p| p.assessment.category == "suggested")
                .map(|p| p.pid)
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(ps[1].assessment.category, "active");
        assert_eq!(ps[2].assessment.category, "keep");
        assert_eq!(ps[3].assessment.category, "keep");
        assert_eq!(ps[4].assessment.category, "observing");
        assert_eq!(ps[5].assessment.category, "observing");
        assert_eq!(ps[6].assessment.category, "keep");
        assert_eq!(ps[7].assessment.reason, "dependent_process");
        assert_eq!(ps[9].assessment.category, "protected");
    }
    #[test]
    fn uncertain_attached_tasks_need_review_and_are_never_assumed_useful() {
        let mut p = candidate(1, None);
        p.orphan = false;
        p.purpose = "unknown".into();
        assess(std::slice::from_mut(&mut p), &HashSet::new(), 1000);
        assert_eq!(p.assessment.category, "review");
        assert_eq!(p.assessment.reason, "check_task");
    }
    #[test]
    fn successful_zero_io_samples_establish_idle_but_failed_reads_reset_it() {
        let mut old = candidate(1, None);
        old.observation.disk_read = 0;
        old.observation.disk_written = 0;
        let mut current = old.clone();
        observe(&mut current, Some(&old), 1005);
        assert!(current.observation.io_known);
        assert_eq!(current.observation.idle_since, 600);
        assess(std::slice::from_mut(&mut current), &HashSet::new(), 1005);
        assert_eq!(current.assessment.category, "suggested");
        for unreadable_prior in [false, true] {
            let mut prior = old.clone();
            let mut next = old.clone();
            if unreadable_prior {
                prior.observation.io_readable = false;
            } else {
                next.observation.io_readable = false;
            }
            observe(&mut next, Some(&prior), 1005);
            assert!(!next.observation.io_known);
            assert_eq!(next.observation.idle_since, 0);
            assert_eq!(next.observation.idle_samples, 0);
        }
    }
    #[test]
    fn fresh_io_cpu_gaps_and_pid_reuse_reset_the_quiet_window() {
        let prior = candidate(1, None);
        let mut same = prior.clone();
        same.observation.idle_since = 0;
        same.observation.idle_samples = 0;
        observe(&mut same, Some(&prior), 1005);
        assert_eq!(same.observation.idle_since, 600);
        for mode in 0..4 {
            let mut p = prior.clone();
            p.observation.idle_since = 0;
            p.observation.idle_samples = 0;
            if mode == 0 {
                p.observation.disk_read += 1
            };
            if mode == 1 {
                p.cpu = 90.
            };
            if mode == 2 {
                p.start_time += 1
            };
            observe(&mut p, Some(&prior), if mode == 3 { 1100 } else { 1005 });
            assert_eq!(p.observation.idle_since, 0);
            assess(std::slice::from_mut(&mut p), &HashSet::new(), 1005);
            assert_ne!(p.assessment.category, "suggested");
        }
    }
}
