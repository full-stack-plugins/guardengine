/// 工具的真实版本和提示；专业上下文由工具自身提供。
pub struct Tool {
    /// 工具名称。
    pub name: &'static str,
    /// 当前二进制版本。
    pub version: &'static str,
    /// 专业能力边界与使用提示。
    pub guidance: &'static str,
    /// 已实现的专业命令；空列表不代表检查通过。
    pub domain_commands: &'static [&'static str],
    /// 可选只读专业上下文提供者。
    pub context: Option<crate::ContextProvider>,
    /// 已实现的专业 MCP；默认不暴露尚未实现的领域能力。
    pub mcp_tools: &'static [crate::DomainTool],
}

impl Tool {
    /// 根据名称、版本、指导与已实现命令创建工具描述，返回不可变描述。
    pub const fn new(
        name: &'static str,
        version: &'static str,
        guidance: &'static str,
        domain_commands: &'static [&'static str],
    ) -> Self {
        Self {
            name,
            version,
            guidance,
            domain_commands,
            context: None,
            mcp_tools: &[],
        }
    }
}
