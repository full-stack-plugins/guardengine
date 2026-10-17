use crate::{PolicyArtifactRef, PolicyBinding, PolicyDecision, PolicyProcessObservation};
use guard_protocol::Digest;
use serde::Serialize;
/// 通用策略求值的本地观察；故障保持Incomplete，绝不签发Grant。无Java对应类型。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyEvaluation {
    pub schema_version: &'static str,
    pub binding: PolicyBinding,
    pub input_digest: Digest,
    pub policy_digest: Digest,
    pub config_digest: Digest,
    pub decision: Option<PolicyDecision>,
    pub incomplete: bool,
    pub version: PolicyProcessObservation,
    pub execution: PolicyProcessObservation,
    pub diagnostics: Vec<String>,
    pub input: PolicyArtifactRef,
    pub policy_sources: Vec<PolicyArtifactRef>,
    pub trust: &'static str,
}
