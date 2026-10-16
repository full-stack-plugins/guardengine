use serde::{Deserialize, Serialize};
/// 受管验证器的执行预算，不允许零预算冒充执行。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecutionBudget {
    /// 冻结协议字段 `timeoutMs`；语义见本 crate 协议表。
    pub timeout_ms: u64,
    /// 冻结协议字段 `maxOutputBytes`；语义见本 crate 协议表。
    pub max_output_bytes: u64,
    /// 冻结协议字段 `maxConcurrency`；语义见本 crate 协议表。
    pub max_concurrency: u32,
}
