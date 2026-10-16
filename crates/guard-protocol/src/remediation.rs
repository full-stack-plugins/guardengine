use crate::Identifier;
use serde::{Deserialize, Serialize};
/// 仅供阅读的修复建议和结构化参数，不自动执行。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Remediation {
    /// 冻结协议字段 `summary`；语义见本 crate 协议表。
    pub summary: Identifier,
    /// 冻结协议字段 `argv`；语义见本 crate 协议表。
    pub argv: Vec<Vec<Identifier>>,
}
