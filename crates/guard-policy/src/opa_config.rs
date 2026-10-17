use crate::PolicyPackage;
use guard_protocol::{Digest, ExecutionBudget, Identifier};
use guard_runner::ProcessRunner;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
/// 显式选择固定OPA原件、版本、环境和预算；不自行安装或接受OPA环境覆盖。无Java对应类型。
#[derive(Clone, Debug)]
pub struct OpaConfig {
    pub executable: PathBuf,
    pub executable_digest: Digest,
    pub version: Identifier,
    pub environment: BTreeMap<String, String>,
    pub budget: ExecutionBudget,
}
impl OpaConfig {
    /// 固定logical argv、包身份、能力、版本、原件、环境与预算；不含随机私有路径。
    pub fn configuration_digest(&self, package: &PolicyPackage) -> Result<Digest, String> {
        self.validate()?;
        crate::policy_package::json_hash(
            &serde_json::json!({"schemaVersion":"guard.opa-profile/v1","artifactDigest":self.executable_digest,"version":self.version,"packageDigest":package.digest(),"entrypoint":package.manifest().entrypoint,"capabilitiesDigest":package.manifest().capabilities.digest,"versionArgv":["version"],"argvTemplate":["eval","--format=json","--strict","--strict-builtin-errors","--fail","--target=rego","--capabilities={{capabilities}}","--bundle={{policy_root}}","--input={{input}}",format!("--timeout={}ms",self.budget.timeout_ms),package.manifest().entrypoint.clone()],"environment":self.environment,"budget":self.budget}),
        )
    }
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.version.as_str() != "1.21.2" || !self.executable.is_absolute() {
            return Err("only explicitly pinned OPA1.21.2 native executable supported".into());
        }
        let mut names = BTreeSet::new();
        for (key, value) in &self.environment {
            let upper = key.to_ascii_uppercase();
            if upper.starts_with("OPA_")
                || !names.insert(upper)
                || key.is_empty()
                || !key.bytes().enumerate().all(|(i, b)| {
                    b == b'_' || b.is_ascii_alphabetic() || (i > 0 && b.is_ascii_digit())
                })
                || value.contains('\0')
            {
                return Err(
                    "explicit ASCII environment without OPA flag overrides/case aliases required"
                        .into(),
                );
            }
        }
        ProcessRunner::new(self.budget.clone())?;
        Ok(())
    }
}
