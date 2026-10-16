use crate::MaterializationReceipt;
use guard_protocol::Digest;
use serde::Serialize;
/// 批准逻辑配置到本次有效执行配置的绑定；无Java对应类型。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvocationReceipt {
    pub profile: &'static str,
    pub approved_config_digest: Digest,
    pub subject_digest: Digest,
    pub effective_invocation_digest: Option<Digest>,
    pub actual_environment_digest: Option<Digest>,
    pub working_directory: Option<String>,
    pub materialization: Option<MaterializationReceipt>,
}
