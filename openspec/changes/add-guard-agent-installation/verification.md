# 验证记录（2026-10-15）

## 范围与环境

- Windows PowerShell，Rust/Cargo 1.99；六个工具均使用真实编译二进制。
- Cargo 缓存：`E:\workspaces\workspace-agent-skills\.guard-build\cargo`。
- 构建输出：`E:\workspaces\workspace-agent-skills\.guard-build\target`。
- TEMP/TMP：`E:\workspaces\workspace-agent-skills\.guard-build\tmp`；测试使用独立项目与模拟用户目录，不写开发者真实宿主配置。
- Python：宿主已有 Python 解释器，使用 `-B`；GitFlow 适配器还使用 `-I -X utf8`。
- 没有创建/切换用户仓库分支，没有提交或推送。测试中只初始化临时 Git 仓库。

## 已执行结果

| 验证 | 结果 |
|---|---|
| 六个 CLI 构建 | 全部成功；CodeGuard 使用明确的 `--no-default-features` 仅接入模式 |
| 共享安装器 Rust 测试 | 3 项通过：先写失败测试，再验证幂等、预览与后续冲突回滚 |
| 六工具真实进程验收脚本 | 9 项通过，最后一次运行 23.793 秒 |
| GuardEngine 原有 `cargo test --all-targets --locked` | 10 项通过 |
| ArchGuard 原有 `cargo test --all-targets --locked` | 5 项通过，实际 CLI ALLOW=0、BLOCK=2 保留 |
| CodeGuard crate_boundaries / workspace_msrv | 10 项通过；包括 CLI 专属 guard-install 依赖约束 |
| Clippy | 共享安装器与五个独立工具 all-targets、CodeGuard 仅接入 bin 均以 `-D warnings` 通过 |
| 格式 | 共享安装器、五工具及 CodeGuard 本次新/改入口与目录文件通过 rustfmt 检查 |
| OpenSpec | `openspec validate add-guard-agent-installation --strict` 通过 |

合计 37 项通过（其中 Python 9 项是跨工具场景测试，每项包含多个真实进程断言）。MSRV 测试核对依赖声明，不代表实际用 Rust 1.85 编译；本机实际编译器为 1.99。

真实进程验收覆盖：

1. 六工具同时写入三个宿主，重复安装无变更，逐一卸载保留其他工具、CRLF 人工内容与旧 CODEGUARD 区块。
2. 未受管的同名 MCP 配置、残缺标记、被编辑区块整体拒绝，无部分落盘。
3. 领域能力未实现时拒绝 check；CodeGuard 仅接入版明确标记版本和能力。
4. GitHub Flow / classic GitFlow 生效角色与提示差异、候选漂移、重新激活后的同步，以及按实际生成配置启动 GitGuard MCP。
5. Windows junction 指向外部配置时拒绝安装，并保留外部文件。
6. MCP 初始化顺序、工具发现、两个工具调用、绝对项目参数与 CLI 返回一致；查询不写文件。
7. 用户级范围、dry-run 不建目录、复制二进制可运行、默认保留与显式删除副本。
8. 未知参数、已有锁、未知二进制目标保护。
9. 项目安装记录中的适配器路径不会被执行。

复现命令（先设置上面的 Cargo/TEMP 环境变量）：

```powershell
cargo test --manifest-path crates/guard-install/Cargo.toml --locked --features test-fixture
cargo test --all-targets --locked
cargo test --manifest-path ../archguard/Cargo.toml --locked --all-targets
cargo test --manifest-path ../codeguard/Cargo.toml -p codeguard-cli --no-default-features --locked --test crate_boundaries --test workspace_msrv
python -B scripts/verify_guard_installation.py --bin-dir E:\workspaces\workspace-agent-skills\.guard-build\target\debug --gitflow-root ../gitflow-plugin
openspec validate add-guard-agent-installation --strict
```

## 失败、限制与发布前事项

- CodeGuard 默认完整构建在本机 Windows 失败，错误位于原有依赖 Unix 的实现；没有将其改写为假通过。完整专业 CLI 从 main.rs 移到 domain_cli.rs，逐行比较确认除入口函数 `main → run` 外正文相同。默认完整构建的 Linux 回归仍需 CI 执行。
- `codeguard/scripts/check_layering.py` 仍失败：finding_identity.rs（89 行）、run_config.rs（135 行）、typed_module_graph.rs（79 行）。对 HEAD 版本运行同一分类逻辑，确认三个结果逐项一致；本次不修改这三处历史领域分层问题或放宽检查脚本。
- 已加入 Ubuntu/Windows CI 配置，但没有把本机验证表述为远端 CI 已运行。相邻 guardengine 源码依赖需要协调发布；公开制品前应固定发布版本。
- 未验证真实 Codex、Claude Code、Gemini CLI 的重新加载和宿主权限确认，也没有安装到开发者真实全局环境。
- GitFlow 已验证普通临时仓库、两类 profile 与漂移；linked worktree、组织继承等沿用原加载器，未在本次 Windows 验收中穷举。
- 本次完成接入层，未实现 FlowGuard/SpecGuard/TestGuard 的专业判定引擎或 GitGuard 写操作。MCP 仅查询；没有授予审批或可信门禁结论。

当前 change 保留为活动状态，实施任务已核对；没有同步/归档为“生产验收完成”。后续发布前完成默认完整构建的 Linux CI、版本化共享依赖和真实宿主验收。
