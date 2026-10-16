# 六个 Guard 的智能体安装与规则提示

## Why

ArchGuard、CodeGuard、FlowGuard、GitGuard、SpecGuard、TestGuard 缺少一致的命令入口与宿主接入方式。智能体需要知道真实可用能力、触发时机与项目规则来源，不能把某个 GitFlow 模板写成所有项目的全局要求。

## What Changes

- 在独立 `guard-install` crate 中实现 CLI 安装、三宿主提示/MCP 配置、所有权记录、预览、同步、诊断与卸载。
- 六个 CLI 复用同一机制，专业能力仍由各工具拥有；已有命令保持兼容。
- 提供只读 MCP context/instructions 工具；不虚构尚未实现的专业检查。
- GitGuard 通过显式配置的 GitFlow 适配器读取真实生效规则，复用其漂移、worktree 和组织规则校验。

## Capabilities

### New Capabilities
- `guard-agent-installation`: 跨工具安装、受管提示、只读 MCP 与规则上下文。

## Impact

跨仓库变更唯一规格事实源为本 change。涉及 guardengine 的独立共享 crate、六个工具的入口/文档/测试；不自动安装到开发者真实宿主，不修改 Git 分支，不迁移现有插件或项目规则。新增四个 CLI 的安装能力不等于专业运行时已完成。
