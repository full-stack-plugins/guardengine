# 第一批架构工程方法整合

## Why
用户已批准 docs/matt-pocock-skills-integration.md 的第一批：公共技能依赖管理、已有规格适配、ArchGuard 五项技能，以及 Java/Maven 检查修复复验闭环，保留 Cargo。

## What Changes
- 公共技能库锁定 Matt Pocock 1.3.1 的提交、MIT 许可、依赖及文件摘要；生成独立可移植 ArchGuard 技能包，校验缺失依赖、冲突与内容漂移。
- 共享只读规格适配器发现 OpenSpec、Spec Kit、已有 Superpowers 产物；歧义不猜测，不初始化体系、不复制规格。
- ArchGuard 可显式绑定原生规格与规则；报告绑定规格快照，规格变化使旧报告失效。
- 集成 domain-modeling、grill-with-docs、codebase-design、improve-codebase-architecture、research 及 grilling 依赖。
- Maven 增加显式 profile 选择的有界静态分析；所有影响模型的 profile 必须明确选择，仍拒绝未知外部模型。真实 CLI/MCP 验证修复闭环，保留 Cargo 回归。

## Scope
本 change 是第一批唯一增量规格，覆盖 guardengine 公共模块与 archguard/plugin 适配。此前 add-maven-architecture-loop 的验收范围不变；本变更扩展其能力。FlowGuard 已有运行时不复制、不替换；共享上下文与原生规格引用供其消费。

## Non-goals
完整 Maven effective model、下载或执行 Maven 插件、Java 类型/方法分析、自动批准架构规则、自动改写业务源码、远程发布、真实宿主安装、其他 22 项技能执行编排。
