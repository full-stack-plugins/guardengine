//! six-guard-context/v1 组织约定：安装器和检查器共用，不属于 AGENTS.md 官方强制字段。

/// 本组织约定包含的工具名称；不代表这些工具已经安装或执行。
pub const CONTEXT_GUARDS: [&str; 6] = [
    "archguard",
    "codeguard",
    "flowguard",
    "gitguard",
    "specguard",
    "testguard",
];

/// 根据工具名返回五条必要指引；未知工具返回空列表。无 Java 对等类型。
pub fn agent_context_requirements(name: &str) -> Vec<String> {
    let trigger = match name {
        "archguard" => "修改模块依赖、分层边界或领域模型前，查询架构契约并执行适用架构检查。",
        "codeguard" => "修改源码及提交审查前，查询代码规范并执行适用质量和安全检查。",
        "flowguard" => "进入交付阶段或变更审批状态前，查询流程门禁与所需证据。",
        "gitguard" => "创建分支、提交、合并或发布前，查询当前 Git 工作流规则。",
        "specguard" => "开始需求、设计或实现前，检查 AGENTS.md 约定并确认唯一规格事实源。",
        "testguard" => "修改行为及交付前，查询测试义务并执行适用测试验证。",
        _ => return Vec::new(),
    };
    vec![
        format!("触发：{trigger}"),
        format!("入口：先运行 {name} context --project PROJECT 获取真实能力和当前规则；PROJECT 替换为实际项目绝对路径。MCP 可用时可调用 {name}_context。"),
        "规则：以项目当前生效的配置、契约和唯一规格事实源为准；仅调用当前版本实际提供的专业检查，不自动初始化或复制第二套规则。".into(),
        "失败处理：工具不可用、未配置、覆盖不完整或检查未通过时，报告阻塞；不得跳过、虚构通过或擅自放宽规则。".into(),
        "证据：修改后重新执行适用检查并核对结果；安装成功、提示已加载、文档合规不代表检查通过或审批授权。".into(),
    ]
}

/// 生成只含组织约定的最小块；输入工具名，返回 Markdown，不执行检查或安装。
pub fn agent_context_block(name: &str) -> String {
    let marker = name.to_uppercase();
    let lines = agent_context_requirements(name);
    if lines.is_empty() {
        return String::new();
    }
    format!(
        "<!-- {marker}_START -->\n## {name}\n\n{}\n<!-- {marker}_END -->\n",
        lines
            .iter()
            .map(|s| format!("- {s}"))
            .collect::<Vec<_>>()
            .join("\n")
    )
}
