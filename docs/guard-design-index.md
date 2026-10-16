# Guard 工具与插件设计文档

## 目标设计

- [ArchGuard 内核、CLI、MCP 与架构契约设计](archguard-cli-mcp-contract-design.md)：独立内核、平台/EA 模型交换、契约与报告、插件开发流程。拟议格式和命令不能当作已实现接口。
- [六个 Guard 的安装与智能体提示注入设计](guard-installation-instruction-design.md)：共享安装器、宿主适配、受管提示、动态项目规则、安装和启用边界。
- [Matt Pocock 技能与六个 Guard 插件整合方案](matt-pocock-skills-integration.md)：27 项能力映射、规格与授权适配、分批实施。

## 已实现能力与使用说明

- [专业 Guard 并行批次与验收](../openspec/changes/professional-guard-checks/verification.md)
- [GitGuard 激活策略检查](../../gitguard/plugin/README.md)
- [FlowGuard 原生阶段只读预检](../../flowguard/plugin/README.md)
- [SpecGuard 原生规格与漂移复验](../../specguard/docs/native-specs.md)
- [TestGuard 导入 JUnit 报告核验](../../testguard/README.md)
- [真实执行、追踪与受信证据增量](../openspec/changes/execution-trace-evidence/verification.md)
- [TestGuard Maven/Cargo 执行器](../../testguard/docs/execution.md)
- [SpecGuard 原生需求与执行用例追踪](../../specguard/docs/trace-evidence.md)

- [ArchGuard Java 源码包依赖、分层与领域类型归属](../../archguard/docs/java-source.md)：0.5.0 可选 Java 策略、需求增量、修复复验和基线更新。

- [安装使用说明](agent-installation.md)
- [ArchGuard 架构规范与检查闭环](archguard-architecture-check-loop.md)
- [ArchGuard 0.3.0 方法与规格绑定指南](../../archguard/docs/method-integration.md)
- [公共技能依赖库](../skill-library/README.md)
- [专业能力路线](professional-roadmap.md)
- [SpecGuard AGENTS.md 组织约定检查](../../specguard/docs/agents-context.md)
- [ArchGuard 0.4.0 项目模型、需求增量与基线更新](../../archguard/docs/project-model.md)

## 第一批规格与验证

- [唯一增量规格](../openspec/changes/integrate-architecture-methods/specs/architecture-method-integration/spec.md)
- [实施任务](../openspec/changes/integrate-architecture-methods/tasks.md)
- [验证记录](../openspec/changes/integrate-architecture-methods/verification.md)
