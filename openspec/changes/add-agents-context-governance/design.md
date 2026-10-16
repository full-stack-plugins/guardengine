# 设计

此变更为跨仓库唯一规格源，SpecGuard 不另行初始化 SDD。

官方基线来自 https://agents.md/ （2026-10-16 核对）：普通 Markdown，无强制字段/标题；根目录及嵌套文档是组织方式。六 Guard 标记、固定指引和 UTF-8/读取预算属于本工具的组织约定或分析边界，不冒称官方要求。

默认组织 profile 为 six-guard-context/v1：指定项目目录必须有 AGENTS.md，并含六个唯一且闭合、不嵌套的 Guard 块。每块包含共享库定义的触发、入口、规则、失败处理、证据五条指引。共享 renderer 和 checker 使用同一份定义；工具原有动态规则摘要保留。

`.specguard/agents-policy.json` 是可选附加约定：schema_version=1、required_text=[{id,text}]，只增加正文中的字面义务，不替换默认六工具规则。拒绝未知字段/版本、空值和重复 ID。它不构成被保护的审批基线，CI 应从可信来源校验策略版本。

CommonMark 解析排除代码块、引用块及 HTML 注释中的示例；只承认独立行的受管标记。输出路径、行号、来源类别、修复提示、文件和策略摘要。检查不执行文档命令，不验证任意自然语言约定的语义一致性。

第一版范围为显式 --project 目录的 AGENTS.md，不自动合并父目录、嵌套 AGENTS.md、AGENTS.override.md 或宿主全局配置；报告明确这些未覆盖项。需要检查子项目时显式指定其目录。缺失/违规返回 BLOCK(2)，读取、解析及配置错误返回 INDETERMINATE(3)，参数错误为4。ALLOW(0)仅表示该文档满足组织约定，运行检查和审批始终 not_evaluated。

agents template 只向 stdout 生成六个最小合规块，不写文件或安装工具。修复使用各工具现有 instructions sync / install 的所有权、冲突、预览机制；不创建第二个 AGENTS.md 写入器。修改后重新 agents check，以当前文件摘要作复验证据。
