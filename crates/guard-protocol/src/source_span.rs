use crate::GitPath;
use serde::{Deserialize, Serialize};
/// 无损 Git 路径上的一基源码位置。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceSpan {
    /// 冻结协议字段 `path`；语义见本 crate 协议表。
    pub path: GitPath,
    /// 冻结协议字段 `startLine`；语义见本 crate 协议表。
    pub start_line: u32,
    /// 冻结协议字段 `startColumn`；语义见本 crate 协议表。
    pub start_column: u32,
    /// 冻结协议字段 `endLine`；语义见本 crate 协议表。
    pub end_line: u32,
    /// 冻结协议字段 `endColumn`；语义见本 crate 协议表。
    pub end_column: u32,
}
