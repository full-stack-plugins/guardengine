use crate::LayerLevel;
use guard_protocol::Identifier;
use serde::Serialize;
/// 可机读阻塞位置；不回显私有来源原文。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilationDiagnostic {
    /// 稳定错误代码。
    pub code: String,
    /// 涉及的层次。
    pub layer: Option<LayerLevel>,
    /// 涉及规则ID。
    pub rule_id: Option<Identifier>,
    /// 有界说明或JSON Pointer。
    pub detail: String,
}
