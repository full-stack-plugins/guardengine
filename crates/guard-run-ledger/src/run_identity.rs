use guard_protocol::Digest;
use serde::{Deserialize, Serialize};
/// 本次执行的完整输入身份声明；不会由本地摘要升级信任。无Java对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunIdentity {
    pub binding_digest: Digest,
    pub subject_digest: Digest,
    pub contract_digest: Digest,
    pub policy_digest: Digest,
    pub verifier_digest: Digest,
    pub environment_digest: Digest,
}
