# 设计

## 边界

共享安装 crate 与 GuardEngine 判定内核没有依赖关系。工具传入名称、版本、专业提示与上下文提供者。CLI、MCP 和生成提示使用相同能力说明。项目规则不写进用户级提示。

## 命令

`install`、`uninstall`、`instructions render|sync`、`context`、`doctor --integration`、`serve --mcp`。支持 `--project`、`--scope project|user`、`--agents codex,claude,gemini`、`--home`、`--bin-dir`、`--dry-run`；JSON 为管理命令稳定输出。install 默认项目级，必须显式选择宿主；bin-dir 可选，指定时复制当前可执行文件，不修改 shell 配置或系统 PATH，返回 PATH 设置提示。uninstall 默认保留二进制，显式 `--remove-binary` 删除归属已验证的副本。

Codex 使用 AGENTS.md 和 TOML；Claude 使用 CLAUDE.md 和 MCP JSON；Gemini 使用 GEMINI.md 和 settings JSON。用户级 CODEX_HOME 按宿主约定解析。CLI 安装可独立于 MCP/提示（agents=none）。

## 文件事务

先生成完整计划并检测冲突，再持有范围共享锁，逐文件验证原字节并同目录临时文件替换。失败时仅回滚仍等于本次写入内容的文件；并发外部修改保留且报告。跨文件进程崩溃不承诺原子性，doctor/重试暴露残留锁或所有权不一致。受管区块外字节保持原样；配置只增删本工具节点。用户改过受管内容、重复/残缺 marker、非普通文件、符号链接/重解析点或未知同名 MCP 项拒绝覆盖。

## 项目上下文

提示内容由工具描述和只读上下文生成，带规则身份与版本。全局只含条件性发现指引。context 查询不初始化项目；未配置与专业能力未实现明确返回 incomplete/not_evaluated。GitGuard 使用用户显式提供的 GitFlow 根和 Python 可执行文件调用原规则加载器，不自行解析/激活另一套规则。候选漂移或不支持格式不能伪装生效。模板字符串作为数据转义，不能注入标记或新指令。

## MCP

stdio JSON-RPC 提供 initialize/ping/tools/list/tools/call。只公开 `<guard>_context` 与 `<guard>_instructions`，要求调用方给出目标项目；无专业 MCP 检查承诺。安装成功、协议测试通过、真实宿主重新加载分别报告。

## 兼容性

保留 ArchGuard check 与 CodeGuard 原有 init/doctor/capabilities；CodeGuard 仅 `doctor --integration` 走新诊断。现有 CODEGUARD:BEGIN project-context 不变。GitFlow/FlowGuard 插件现有规则、路径和执行器不迁移。共享 crate 暂用同级源码依赖，发布前必须版本化。

## 审查后补充的执行边界

项目安装记录不能授权执行适配器。CLI 必须显式传入可信 GitFlow 根和 Python；MCP 由安装时生成的宿主配置固定参数。更新和同步重复显式参数。二进制跨位置更新重复 --bin-dir；删除副本同时要求 --remove-binary 与匹配的 --bin-dir。

CodeGuard 默认 full-cli 特性保持原专业入口；--no-default-features 明确构建仅接入版本。此模式用于当前 Windows 接入验证，不声称完成完整 CodeGuard Windows 移植。旧专业 mcp serve 规划状态保留，新 serve --mcp 仅查询上下文。

可视化设计、宿主映射、部署顺序和示例见 docs/agent-installation.md。共享 crate 使用 path 依赖，六仓库 CI 必须先取得包含本变更的 guardengine 源码；真实宿主验收和远端 CI 运行独立记录。
