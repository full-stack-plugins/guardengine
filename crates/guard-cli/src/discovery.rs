use crate::{host_config::HostConfig, safe_files};
use serde_json::{Value, json};
use std::path::Path;

/// 引导失败时只报告本进程事实；参数为同一失败原因，返回未接触候选或工具的各发现维度。
pub(crate) fn unavailable(reason: &str) -> Value {
    let not_inspected = json!({"status":"not_inspected","reason":reason});
    json!({
        "cli":{"status":"observed","name":"guardcore","version":env!("CARGO_PKG_VERSION"),"executable":std::env::current_exe().ok()},
        "platform":{"status":"observed","os":std::env::consts::OS,"arch":std::env::consts::ARCH,"fullEnvironment":"unknown"},
        "skills":not_inspected,"projectInitialization":not_inspected,"languageBuild":not_inspected,
        "git":not_inspected,"verifiers":not_inspected
    })
}

/// 只报告独立观察的发现面；不由CLI存在推断Skills或项目初始化。无Java对应对象。
pub(crate) fn observe(host: &HostConfig) -> Result<Value, String> {
    let project = host.project()?;
    let markers = [
        ".specify",
        "openspec",
        "docs/superpowers/specs",
        "docs/superpowers/plans",
    ];
    let builds = [
        "Cargo.toml",
        "pom.xml",
        "build.gradle",
        "build.gradle.kts",
        "package.json",
        "pyproject.toml",
    ];
    Ok(json!({
        "cli": {"status":"observed", "name":"guardcore", "version":env!("CARGO_PKG_VERSION"), "executable":std::env::current_exe().map_err(|e|e.to_string())?},
        "skills": {"status":"not_inspected", "reason":"no protected skills inventory provider configured"},
        "projectInitialization": {"status":"observed_markers_only", "markers":markers.iter().map(|name|marker(&project,name)).collect::<Vec<_>>()},
        "languageBuild": {"status":"observed_markers_only", "markers":builds.iter().map(|name|marker(&project,name)).collect::<Vec<_>>(), "semanticCompatibility":"not_inspected"},
        "platform": {"status":"observed", "os":std::env::consts::OS, "arch":std::env::consts::ARCH, "fullEnvironment":"unknown"},
        "git": {"status":"configured_artifact_checked", "providerDigest":crate::prepared::git_identity(host)?, "actualCaptureTestedByDoctor":false, "helpersAndLibraries":"unknown"}
    }))
}

fn marker(project: &Path, name: &str) -> Value {
    let path = project.join(name);
    let status = match std::fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => "absent",
        Err(_) => "unknown",
        Ok(_) => match safe_files::plain(&path) {
            Ok(_) => "present",
            Err(_) => "unsafe_or_unavailable",
        },
    };
    json!({"path":name,"status":status})
}
