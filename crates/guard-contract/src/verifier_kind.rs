use serde::Serialize;
/// 验证器的内核职责与专业域职责分离。无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifierKind {
    /// 专业检查器，需要六个专业 owner 之一。
    Domain,
    /// OPA 策略引擎，不冒充专业域 owner。
    PolicyEngine,
}
