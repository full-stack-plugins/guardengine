use crate::SymbolReference;
use serde::{Deserialize, Serialize};
/// 带解析限制的依赖方向，不具有放行权。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DependencyEdge {
    /// 冻结协议字段 `from`；语义见本 crate 协议表。
    pub from: SymbolReference,
    /// 冻结协议字段 `to`；语义见本 crate 协议表。
    pub to: SymbolReference,
}
