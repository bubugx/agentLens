//! Explicit ownership rules. Runtime bindings are deliberately never persisted.
use crate::{collector::AGENTS, model::Root};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessRule {
    pub executable: String,
    /// Absolute script path, or `module:package.name` for Python -m.
    pub entrypoint: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDefinition {
    pub id: String,
    pub name: String,
    pub color: String,
    pub builtin: bool,
    pub enabled: bool,
    pub rule: Option<ProcessRule>,
}
#[derive(Clone, Debug)]
pub struct AgentBinding {
    pub agent: String,
    pub pid: u32,
    pub start_time: u64,
    pub executable: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessCandidate {
    pub pid: u32,
    pub parent_pid: Option<u32>,
    pub start_time: u64,
    pub name: String,
    pub executable: String,
    pub working_directory: String,
    pub entrypoint: String,
    pub memory: u64,
    pub cpu: f32,
    pub agent: Option<String>,
    pub can_remember: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryBinding {
    pub path: String,
    pub usage: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptRequest {
    pub name: String,
    pub color: String,
    pub pid: Option<u32>,
    pub start_time: Option<u64>,
    pub executable: String,
    pub entrypoint: String,
    pub remember: bool,
    pub directories: Vec<DirectoryBinding>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptionPreview {
    pub processes: Vec<ProcessCandidate>,
    pub roots: Vec<Root>,
    pub rule: Option<ProcessRule>,
}
pub fn definitions(custom: &[AgentDefinition]) -> Vec<AgentDefinition> {
    let colors = [
        "#cd8b70", "#528f80", "#8584b9", "#7aa4c5", "#c6aa6f", "#9b9ca1", "#8497a7",
    ];
    AGENTS
        .iter()
        .enumerate()
        .map(|(i, (id, name))| AgentDefinition {
            id: (*id).into(),
            name: (*name).into(),
            color: colors[i].into(),
            builtin: true,
            enabled: true,
            rule: None,
        })
        .chain(custom.iter().cloned())
        .collect()
}
pub fn is_runtime(executable: &str) -> bool {
    let name = Path::new(executable)
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    matches!(
        name.as_str(),
        "node"
            | "nodejs"
            | "python"
            | "python3"
            | "pythonw"
            | "ruby"
            | "perl"
            | "php"
            | "sh"
            | "bash"
            | "zsh"
            | "fish"
            | "cmd"
            | "powershell"
            | "pwsh"
            | "bun"
            | "deno"
            | "java"
            | "electron"
            | "uv"
            | "npx"
            | "npm"
            | "pnpm"
            | "yarn"
    ) || name.starts_with("python3.")
}
/// Extract only the invocation's entry, never arbitrary arguments (which may contain secrets).
pub fn entrypoint(executable: &str, args: &[String], cwd: &str) -> String {
    if !is_runtime(executable) {
        return String::new();
    }
    let mut tokens = args.iter().skip(1);
    let Some(first) = tokens.next() else {
        return String::new();
    };
    if first == "-m"
        && Path::new(executable)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .starts_with("python")
    {
        return tokens
            .next()
            .filter(|s| {
                !s.is_empty()
                    && s.chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
            })
            .map(|s| format!("module:{s}"))
            .unwrap_or_default();
    }
    // Complex shell commands, eval, wrappers and unknown flags require session-only adoption.
    if first.starts_with('-') || first == "run" || first == "exec" {
        return String::new();
    }
    let path = Path::new(first);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else if !cwd.is_empty() {
        Path::new(cwd).join(path)
    } else {
        return String::new();
    };
    std::fs::canonicalize(&absolute)
        .unwrap_or(absolute)
        .to_string_lossy()
        .into_owned()
}
pub fn rule_matches(rule: &ProcessRule, executable: &str, entry: &str) -> bool {
    rule.executable == executable
        && rule.entrypoint == entry
        && (!is_runtime(executable) || !entry.is_empty())
}
pub fn binding_matches(binding: &AgentBinding, pid: u32, start: u64, exe: &str) -> bool {
    binding.pid == pid && binding.start_time == start && binding.executable == exe
}
pub fn validate_rule(rule: &ProcessRule) -> Result<(), String> {
    if !Path::new(&rule.executable).is_absolute() || !Path::new(&rule.executable).is_file() {
        return Err("Choose an existing executable using its absolute path".into());
    }
    if is_runtime(&rule.executable) && rule.entrypoint.is_empty() {
        return Err("A shared runtime requires a specific script or module entry point".into());
    }
    if !rule.entrypoint.is_empty()
        && !rule.entrypoint.starts_with("module:")
        && !Path::new(&rule.entrypoint).is_absolute()
    {
        return Err("Use an absolute script path or module:package.name".into());
    }
    if !rule.entrypoint.is_empty() && !is_runtime(&rule.executable) {
        return Err("A script entry point is only supported for a shared runtime".into());
    }
    if let Some(module) = rule.entrypoint.strip_prefix("module:") {
        if !Path::new(&rule.executable)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .starts_with("python")
            || module.is_empty()
            || !module
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        {
            return Err("Use module:package.name only with a Python executable".into());
        }
    } else if !rule.entrypoint.is_empty() && !Path::new(&rule.entrypoint).is_file() {
        return Err("Choose an existing script entry point".into());
    }
    if rule.entrypoint.len() > 4096 || rule.executable.len() > 4096 {
        return Err("Recognition rule is too long".into());
    }
    Ok(())
}
pub fn validate_directories(
    bindings: &[DirectoryBinding],
    existing: &[Root],
    home: &Path,
    agent: &str,
) -> Result<Vec<Root>, String> {
    if bindings.len() > 20 {
        return Err("Bind at most 20 directories at a time".into());
    }
    let home = std::fs::canonicalize(home).map_err(|e| e.to_string())?;
    let mut roots: Vec<Root> = vec![];
    for b in bindings {
        if !["project", "cache", "logs", "temporary"].contains(&b.usage.as_str()) {
            return Err("Invalid directory usage".into());
        }
        let path = PathBuf::from(&b.path);
        if !path.is_absolute() || path.is_symlink() || !path.is_dir() {
            return Err("Select an existing real directory".into());
        }
        let canonical = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
        if canonical == home || !canonical.starts_with(&home) {
            return Err("Choose a directory inside your home".into());
        }
        if existing
            .iter()
            .chain(roots.iter())
            .any(|r| canonical.starts_with(&r.path) || Path::new(&r.path).starts_with(&canonical))
        {
            return Err("This directory overlaps an existing monitoring root".into());
        }
        roots.push(Root {
            id: uuid::Uuid::new_v4().to_string(),
            path: canonical.to_string_lossy().into_owned(),
            agent: agent.into(),
            label: canonical
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            custom: true,
            source: format!("adopted-{}", b.usage),
        });
    }
    Ok(roots)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interpreter_entry_is_exact_and_does_not_search_arbitrary_arguments() {
        let exe = "/usr/bin/python3";
        let args = |a: &[&str]| a.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let rule = ProcessRule {
            executable: exe.into(),
            entrypoint: "module:my_agent".into(),
        };
        assert!(rule_matches(
            &rule,
            exe,
            &entrypoint(
                exe,
                &args(&[exe, "-m", "my_agent", "--token", "secret"]),
                "/tmp"
            )
        ));
        assert!(!rule_matches(
            &rule,
            exe,
            &entrypoint(exe, &args(&[exe, "-m", "other", "my_agent"]), "/tmp")
        ));
        assert!(!rule_matches(
            &ProcessRule {
                executable: exe.into(),
                entrypoint: "".into()
            },
            exe,
            ""
        ));
        assert_eq!(
            entrypoint(exe, &args(&[exe, "-c", "print('x')", "my_agent"]), "/tmp"),
            ""
        );
        assert_eq!(
            entrypoint(
                "/bin/node",
                &args(&["node", "worker.js"]),
                "/projects/agent"
            ),
            "/projects/agent/worker.js"
        );
    }
    #[test]
    fn runtime_bindings_require_all_identity_fields() {
        let b = AgentBinding {
            agent: "custom".into(),
            pid: 42,
            start_time: 123,
            executable: "/bin/agent".into(),
        };
        assert!(binding_matches(&b, 42, 123, "/bin/agent"));
        assert!(!binding_matches(&b, 42, 124, "/bin/agent"));
        assert!(!binding_matches(&b, 42, 123, "/bin/other"));
    }
    #[test]
    fn directories_are_unique_scoped_and_have_explicit_usage() {
        let home = tempfile::tempdir().unwrap();
        let project = home.path().join("project");
        std::fs::create_dir(&project).unwrap();
        let b = DirectoryBinding {
            path: project.to_string_lossy().into_owned(),
            usage: "cache".into(),
        };
        let roots = validate_directories(&[b.clone()], &[], home.path(), "custom").unwrap();
        assert_eq!(roots[0].source, "adopted-cache");
        assert!(validate_directories(&[b], &roots, home.path(), "other").is_err());
        assert!(validate_directories(
            &[DirectoryBinding {
                path: home.path().to_string_lossy().into_owned(),
                usage: "project".into()
            }],
            &[],
            home.path(),
            "x"
        )
        .is_err());
    }
}
