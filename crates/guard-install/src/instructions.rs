use crate::{Options, Tool};
use serde_json::{Value, json};

/// 将不可信字段编码成单行 JSON 数据，避免形成 Markdown 指令或受管标记。
pub(crate) fn quote(value: &Value) -> String {
    value
        .to_string()
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('`', "\\u0060")
}

pub(crate) fn context(tool: &Tool, options: &Options) -> Result<Value, String> {
    let domain = match tool.context {
        Some(provider) => provider(options)?,
        None => {
            json!({"status":"not_evaluated", "reason":"Use the documented domain commands; installation is not a domain check."})
        }
    };
    let mut mcp_tools = vec![
        format!("{}_context", tool.name),
        format!("{}_instructions", tool.name),
    ];
    mcp_tools.extend(tool.mcp_tools.iter().map(|entry| entry.name.to_owned()));
    Ok(json!({
        "schema_version":1, "tool":tool.name, "version":tool.version,
        "project":options.project, "domain":domain,
        "specification":crate::specification_context(&options.project, options.spec_system.as_deref(), options.spec_target.as_deref())?,
        "domain_commands":tool.domain_commands,
        "management_commands":["install","uninstall","context","spec context","instructions render","instructions sync","doctor --integration","serve --mcp"],
        "mcp_tools":mcp_tools,
        "guidance":tool.guidance, "quality_decision":"not_evaluated"
    }))
}

pub(crate) fn render(tool: &Tool, options: &Options) -> Result<String, String> {
    let marker = tool.name.to_uppercase();
    let mut body = format!(
        "<!-- {marker}_START -->\n## {}\n\n- 在适用项目中，先运行 `{} context --project <实际项目路径>` 获取能力与当前规则；不得因安装而自动初始化项目。\n- MCP 可用时使用 `{}_context` / `{}_instructions`；否则使用 CLI。这些 MCP 工具只读，不执行专业检查。\n- {}\n- 专业检查以当前版本帮助和实际结果为准；安装成功、提示已加载、检查通过与审批授权是不同状态。\n",
        tool.name, tool.name, tool.name, tool.name, tool.guidance
    );
    for requirement in crate::agent_context_requirements(tool.name) {
        body.push_str(&format!("- {requirement}\n"));
    }
    if !tool.domain_commands.is_empty() {
        body.push_str(&format!(
            "- 当前专业 CLI 入口：{}。\n",
            quote(&json!(tool.domain_commands))
        ));
    }
    if !tool.mcp_tools.is_empty() {
        body.push_str(&format!(
            "- 当前专业 MCP 入口：{}；与 CLI 共用判定实现。\n",
            quote(&json!(
                tool.mcp_tools
                    .iter()
                    .map(|entry| entry.name)
                    .collect::<Vec<_>>()
            ))
        ));
    }
    if !options.user {
        let data = context(tool, options)?;
        body.push_str("\n项目上下文（生成摘要，执行前重新查询）：\n");
        let domain = &data["domain"];
        body.push_str(&format!(
            "- 原生规格引用：{}；存在歧义时显式选择，ready 不表示批准，不复制第二份规格。\n",
            quote(&data["specification"])
        ));
        if domain["status"] == "active" && domain["roles"].is_object() {
            body.push_str(&format!(
                "- 工作流：{}；修订：{}；摘要：{}。\n",
                quote(&domain["profile"]),
                quote(&domain["revision"]),
                quote(&domain["policy_sha256"])
            ));
            for (role, details) in domain["roles"].as_object().unwrap() {
                let name = if details["name"].is_null() {
                    &details["pattern"]
                } else {
                    &details["name"]
                };
                body.push_str(&format!(
                    "- 角色 {}：名称/模式 {}；创建来源 {}；允许合入 {}；允许直接提交 {}。\n",
                    quote(&json!(role)),
                    quote(name),
                    quote(&details["create_from"]),
                    quote(&details["merge_into"]),
                    quote(&details["allow_commit"])
                ));
            }
            body.push_str(&format!(
                "- 权威远端 {}；合并方式 {}；提交消息模式 {}。\n",
                quote(&domain["primary_remote"]),
                quote(&domain["merge_method"]),
                quote(&domain["commit_message_pattern"])
            ));
            body.push_str("- 以上是生效快照的可读投影，不是新的规则源；修改规则后须经原工作流激活并同步提示。\n");
        } else {
            body.push_str(&format!("- 状态与规则数据：{}\n", quote(domain)));
        }
    } else {
        body.push_str("\n此用户级指引适用于多个项目；具体分支、阶段、契约和测试义务必须读取各项目生效规则。\n");
    }
    body.push_str(&format!("<!-- {marker}_END -->"));
    Ok(body)
}
