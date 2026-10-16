use serde_json::Value;

/// 工具自身提供的专业 MCP 入口；共享层只负责协议分发，无 Java 对应类。
pub struct DomainTool {
    /// MCP 工具名。
    pub name: &'static str,
    /// 真实能力与范围描述。
    pub description: &'static str,
    /// 返回严格的输入 schema。
    pub schema: fn() -> Value,
    /// 验证参数并执行领域查询；不能隐式批准变更。
    pub call: fn(&Value) -> Result<Value, String>,
}
