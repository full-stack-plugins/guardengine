use crate::LayerLevel;
use guard_protocol::{Digest, Identifier};
use serde::Serialize;
/// 最终编译保留的每层批准身份，不随候选修改原地改变。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LayerIdentity {
    /// 组织、项目或任务层。
    pub level: LayerLevel,
    /// 契约ID。
    pub contract_id: Identifier,
    /// 显式修订版本。
    pub version: Identifier,
    /// 已重算并核验的内容摘要。
    pub digest: Digest,
    /// 对应的批准引用。
    pub approval_ref: Identifier,
}
