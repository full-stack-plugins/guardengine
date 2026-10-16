# AGENTS.md 上下文治理验证

日期：2026-10-16。环境：Windows，构建及隔离测试目录在 E:/workspaces/workspace-agent-skills/.guard-build，未修改实际宿主配置。

## 实现与规格对应

| 要求 | 实现与证据 |
|---|---|
| 官方与组织约定区分 | 报告 official_baseline、profile、finding.origin；官方无强制标题，默认六块属于组织要求 |
| 真实正文和附加义务 | CommonMark 解析，五条指引 × 六工具逐条缺失测试，代码/引用/注释排除，平衡/重复块及自定义策略测试 |
| 失败关闭 | 未知策略字段/版本/重复 ID、空规则、1 MiB 超限、乱码、非普通文件、Windows junction 均不能 ALLOW |
| CLI/MCP 同源只读 | 实际 stdio 握手、发现、check/template 调用、非法参数；与 CLI JSON 相等，文档及目录内容不变 |
| 安装修复复验 | 六真实 CLI 安装生成块通过 SpecGuard；sync 保留人工内容；修改正文触发失败，恢复正文并重跑回到通过 |

## 验证命令和结果

- SpecGuard：`cargo test --locked`，10 项通过；`cargo clippy --all-targets -- -D warnings` 通过。
- 共享安装库：`cargo test --manifest-path crates/guard-install/Cargo.toml --locked --features test-fixture`，8 项通过；对应 clippy 通过。
- 六个实际 CLI 构建成功，CodeGuard 使用 `--no-default-features` 接入构建；它已有 7 条非致命编译 warning，不宣称该工具全量领域构建已验证。
- `python -B scripts/verify_guard_installation.py --bin-dir <构建目录>/debug --gitflow-root ../gitflow-plugin`，10 项通过，涵盖六工具块、三宿主文件、幂等/同步/卸载/冲突、GitFlow 动态规则与真实 MCP。
- ArchGuard：`cargo test --manifest-path ../archguard/Cargo.toml --locked`，23 项通过，保留 Java/Maven 和 Cargo 闭环。
- `openspec validate add-agents-context-governance --strict` 通过。
- 插件 JSON 与官方 1.0.0 Schema 的已使用字段/固定目录逐项核对，并有 Rust 包结构测试；实际 MCP 已运行。环境没有 Python jsonschema / Node Ajv，因此未宣称运行通用 JSON Schema 验证器。未验证 Codex/Claude/Gemini 实际加载插件。

合计 51 项已执行测试通过。最初 5 项测试均因 agents 入口缺失失败，再实现通过；示例测试进一步发现多行 HTML 注释误识别，修复后通过。

## 本地只读审计

`E:/workspaces/workspace-agent-skills/AGENTS.md` 存在。SHA256 为 `2531537c801e6fe98e620271299ded871c484aa5ffbe48a3670919a56675e1e8`，检查返回 BLOCK：缺少 archguard、codeguard、flowguard、gitguard、specguard、testguard 六个受管块。该目录没有 `.specguard/agents-policy.json`。未覆盖或修改现有人工规范。

报告仅表示当前指定根文档的组织约定状态。父子目录继承、宿主覆盖/截断、自然语言语义冲突、二进制实际安装状态、智能体执行和审批不在本检查覆盖范围。Unix symlink 分支测试在本 Windows 环境未运行。

## 规格状态

该增量已实现并验证，保留为 active change 供审阅，未执行 sync/archive；没有提交、发布或安装到真实宿主。策略和文档摘要不构成可信签名或批准；CI/Harness 的强制执行对接仍属后续能力。
