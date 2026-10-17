use guard_protocol::{Digest, DomainOwner, Identifier};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
/// 宿主固定的验证器及只读探测配置；不接受命令行覆写。无Java对应类型。
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ConfiguredVerifier {
    pub verifier_id: Identifier,
    pub version: Identifier,
    pub artifact_digest: Digest,
    pub config_digest: Digest,
    pub adapter_version: Identifier,
    pub kind: String,
    pub owner: Option<DomainOwner>,
    pub evidence_types: Vec<Identifier>,
    pub parameter_schema: serde_json::Value,
    pub executable: PathBuf,
    pub argv: Vec<String>,
    pub environment: BTreeMap<String, String>,
    pub probe_argv: Vec<String>,
    pub probe_environment: BTreeMap<String, String>,
    pub probe_stdout: String,
    pub probe_stderr: String,
}
impl ConfiguredVerifier {
    /// 从固定配置生成注册描述，返回合法种类与领域约束。
    pub(crate) fn descriptor(&self) -> Result<guard_contract::VerifierDescriptor, String> {
        Ok(guard_contract::VerifierDescriptor {
            verifier_id: self.verifier_id.clone(),
            version: self.version.clone(),
            artifact_digest: self.artifact_digest.clone(),
            config_digest: self.config_digest.clone(),
            adapter_version: self.adapter_version.clone(),
            kind: match self.kind.as_str() {
                "domain" => guard_contract::VerifierKind::Domain,
                "policy_engine" => guard_contract::VerifierKind::PolicyEngine,
                _ => return Err("unsupported verifier kind".into()),
            },
            owner: self.owner,
            evidence_types: self.evidence_types.clone(),
            parameter_schema: serde_json::to_vec(&self.parameter_schema)
                .map_err(|e| e.to_string())?,
        })
    }
    /// 按给定目录和探测档位生成显式命令，不运行进程。
    pub(crate) fn command(&self, root: &Path, probe: bool) -> guard_runner::CommandSpec {
        guard_runner::CommandSpec {
            executable: self.executable.clone(),
            executable_digest: self.artifact_digest.clone(),
            args: if probe {
                self.probe_argv.clone()
            } else {
                self.argv.clone()
            },
            directory: root.to_path_buf(),
            environment: if probe {
                self.probe_environment.clone()
            } else {
                self.environment.clone()
            },
        }
    }
    /// 校验版本参数与最小独立环境，返回可探测或拒绝原因。
    pub(crate) fn validate_probe(&self) -> Result<(), String> {
        let allowed = if self.kind == "policy_engine" {
            self.probe_argv == ["version"]
        } else {
            self.probe_argv == ["--version"] || self.probe_argv == ["-version"]
        };
        if !allowed || self.probe_stdout.is_empty() && self.probe_stderr.is_empty() {
            return Err("only explicit nonempty version-output probe profiles supported".into());
        }
        for key in self.probe_environment.keys() {
            if !["SystemRoot", "WINDIR", "LANG", "LC_ALL"].contains(&key.as_str()) {
                return Err(
                    "probe environment must be minimal; loader/project/run overrides refused"
                        .into(),
                );
            }
        }
        Ok(())
    }
}
