# AGENTS.md 上下文规范治理

## Why
六个 Guard 已能安装和注入提示，但 SpecGuard 还不能检查本地 AGENTS.md 是否存在、是否完整包含组织要求，空标记也无法保证指引可用。

## What Changes
- 在共享安装库定义版本化的六 Guard 上下文约定，并由各工具的安装/同步输出。
- SpecGuard 提供只读 agents check / agents template 和同源 MCP；检查文档结构、必要指引及项目附加约定。
- 添加插件技能，形成检查、按现有安装机制修复、重新检查的流程。

## Impact
涉及 guardengine/crates/guard-install、specguard 和六工具接入回归。不改变专业领域规则或授予执行/合入许可；不自动安装、初始化、修改用户 AGENTS.md。
