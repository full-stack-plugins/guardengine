# ArchGuard 架构规范与检查闭环

ArchGuard 首批架构检查以 Java/Maven 为优先目标，同时保留 Cargo 支持。

- 支持直接依赖禁止、间接路径禁止、依赖白名单、必需依赖、循环检查。
- 检查流程为 `contract validate → check → explain → 修复 → verify`；旧报告、源码或契约变化会被识别。
- CLI、专业 MCP、[ArchGuard 插件包](E:/workspaces/workspace-agent-skills/full-stack-plugins-repositories/archguard/plugin/README.md) 共用检查实现，安装提示包含当前项目规则。
- **48 个测试通过**，Clippy、格式及 OpenSpec 严格校验通过。[验证记录](E:/workspaces/workspace-agent-skills/full-stack-plugins-repositories/archguard/openspec/changes/add-maven-architecture-loop/verification.md)

真实 `ddd4j` 检查停在父 POM 的模型相关 profile，明确返回覆盖不足并阻断。后续能力建设优先补齐 Maven 有效模型，再扩展 Java 源级边界与方法检查。

其他五个工具的接入兼容回归已通过，专业能力规划见[六工具路线](E:/workspaces/workspace-agent-skills/full-stack-plugins-repositories/guardengine/docs/professional-roadmap.md)。真实宿主加载与远程 CI 尚未验证，本地测试结果不能替代这两项验收。

