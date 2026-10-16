use crate::{Identifier, SourceSpan};
use serde::{Deserialize, Serialize};
/// 带仓库及源码位置的符号身份。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SymbolReference {
    /// 冻结协议字段 `repositoryId`；语义见本 crate 协议表。
    pub repository_id: Identifier,
    /// 冻结协议字段 `symbolId`；语义见本 crate 协议表。
    pub symbol_id: Identifier,
    /// 冻结协议字段 `sourceSpan`；语义见本 crate 协议表。
    pub source_span: SourceSpan,
}
