use crate::Identifier;
use serde::{Deserialize, Serialize};
/// 精确操作资源，不接受任意 shell 或自由文本命令。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActionResource {
    /// 冻结协议字段 `repositoryId`；语义见本 crate 协议表。
    pub repository_id: Identifier,
    /// 冻结协议字段 `targetRef`；语义见本 crate 协议表。
    pub target_ref: Identifier,
}
