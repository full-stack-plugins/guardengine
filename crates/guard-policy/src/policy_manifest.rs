use crate::PolicyFile;
use guard_protocol::{DomainOwner, Identifier};
use serde::{Deserialize, Serialize};
/// 新增的闭合策略包身份规范；不是ContractSnapshot既有摘要公式。无Java对应类型。
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PolicyManifest {
    pub schema_version: u32,
    pub package_id: Identifier,
    pub version: Identifier,
    pub owner: DomainOwner,
    pub entrypoint: String,
    pub files: Vec<PolicyFile>,
    pub capabilities: PolicyFile,
}
