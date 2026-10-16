# 专业 Guard 并行批次验收

日期：2026-10-16。本 change 为唯一跨仓库增量规格；本地实施与验证完成，未归档、提交或发布。三个实施智能体并发负责 GitGuard、SpecGuard、TestGuard，随后复用一个智能体推进 FlowGuard；主代理负责共享接入、插件包、交叉审查收敛和联合验证。

## 可审阅交付

| 工具 | 实际增量 | 验证记录 |
|---|---|---|
| GitGuard | policy check branch/merge/commit；复用原激活 GitFlow，不执行 Git 写入 | [GitGuard](../../../../gitguard/docs/policy-check-verification.md) |
| SpecGuard | OpenSpec/Spec Kit 原生结构、产物清单与摘要、观察快照漂移复验；保留 AGENTS 检查 | [SpecGuard](../../../../specguard/docs/verification-native-specs.md) |
| TestGuard | JUnit/Surefire 严格子集核验；失败/跳过/重试/计数/必需用例/候选摘要；不签发执行证明 | [TestGuard 审查修复](../../../../testguard/docs/verification-junit-metadata.md) |
| FlowGuard | 复用原 stage_docs.snapshot/missing_before 的状态查询与前置产物预检；资源/路径/漂移保护 | [FlowGuard](../../../../flowguard/docs/stage-check-verification.md) |
| ArchGuard | 保留 0.5.0 项目模型、需求增量、Java/Maven/Cargo 检查闭环，完成兼容回归 | [ArchGuard](../../../../archguard/openspec/changes/java-source-architecture/verification.md) |
| 共享与插件 | 新增四工具领域 MCP 期望、五包结构与 stdio 校验；补齐 GitGuard/FlowGuard/TestGuard 可移植 plugin.json/mcp.json、技能和 CI 入口 | scripts/verify_guard_installation.py、scripts/verify_guard_packages.py |

原 gitflow-plugin、flowguard-plugin 的业务规则未被复制或修改；其既有规格仍约束原生语义。共享 GuardEngine Rust 协议和生产 crate 未改动，所有新增领域入口使用既有 DomainTool 接口。

## 实际执行结果

环境为 Windows，Cargo home/target、所有隔离夹具位于工作区 `.guard-build`。Python 使用现有运行时及 -B。未在真实业务仓库执行测试命令、改 Git 策略、写架构基线或推进阶段。

| 实际命令/检查 | 结果 |
|---|---|
| ArchGuard cargo test --locked --offline | 53 通过，含 Java/Maven 与 Cargo |
| SpecGuard cargo test --locked --offline | 22 通过（AGENTS 10、原生规格 12） |
| TestGuard cargo test --locked --offline | 修复后 29 通过，含真实 stdio MCP |
| GITGUARD_BIN 指向最终联合二进制，python -B tests/policy_check.py | 5 通过，主代理复验 29.088s |
| FLOWGUARD_BIN 指向最终联合二进制，python -B tests/stage_check.py | 4 通过，主代理复验 3.224s |
| 原 flowguard-plugin stage transitions | 实施智能体执行 8 通过，原插件工作树保持干净 |
| 各新增工具 build/fmt/clippy -D warnings | 实施智能体全部通过；Rust 无行为单元测试的 GitGuard/FlowGuard 不把空 cargo test 当作专业证明 |
| python -B scripts/verify_guard_packages.py --repositories-root .. --bin-dir E:/workspaces/workspace-agent-skills/.guard-build/professional-bin | 5 包结构/包内技能链接通过；实际 stdio MCP 工具数量 10/3/3/6/3（Arch/Git/Flow/Spec/Test） |
| python -B scripts/verify_guard_installation.py --bin-dir E:/workspaces/workspace-agent-skills/.guard-build/professional-bin --gitflow-root ../gitflow-plugin | 六工具共存、安装卸载、上下文与 MCP 10 项全部通过，19.736s |
| openspec validate professional-guard-checks --strict | 通过 |
| 各仓库 git diff --check | 通过；既有 LF/CRLF 提示不作为失败 |

联合目录复制的是各智能体最终构建的二进制；ArchGuard/CodeGuard 来自此前共享 target，其他四个来自各自独立 target。CodeGuard 本批仅共享安装/MCP 兼容验证，未重跑全部专业分析器，不声称其完整专业能力重新验收通过。

插件校验脚本检查本批使用的可移植结构子集、相对技能链接和实际 stdio 发现，不是通用完整 JSON Schema 实现，也不证明真实宿主已经加载。CI 已加入相同入口；远程 CI、Linux/macOS 和发布流程未执行，跨仓库源码需协调发布。

## 失败后修复与交叉审查

专业能力先以负例建立失败测试，再实现 CLI/MCP 和修复复验。共享安装回归初次失败原因是新 MCP 未登记；登记真实工具后重新联合执行全部 10 项通过，不删检查规避失败。

交叉审查发现并修复：

1. GitGuard P1：git status 可执行项目 clean/process filter。先用真实 marker 夹具重现，再在 Windows/POSIX 共用传输中读取有效配置，存在可执行 filter 即 unverified，不运行 status；不伪造干净状态。有效 LFS filter 也保守拒绝，这是明确限制。
2. GitGuard P2：Windows 清理全部 GIT_* 导致实际作者邮箱未检查。新回归先证明禁用邮箱被错误允许，修复后保留 AUTHOR/COMMITTER，CLI/MCP 均正确 deny。
3. TestGuard P2：properties/system-out 中嵌入 failure/skipped 被忽略。两条实际失败回归后，严格验证元数据结构；合法 text/CDATA/property 保留，未知结果后代拒绝。
4. FlowGuard 来源边界：有效大文件、外部 docs junction 原可 observed。两项先失败后修复；前置检查原生定位的阶段文件，限制 1MiB/16MiB、拒绝链接/外部/非普通文件，结束后重新核验摘要。

TestGuard 另有深层 XML 实测触发 Windows 栈溢出后修复的记录；现有预检先限制深度/节点再调用 DOM，后续 10 万层及超限探测均返回错误，没有崩溃。

## 不能扩大解释的结果

- GitGuard allow 仅激活 Git 策略通过，不等于质量、审批或合入授权；配置并发修改仍需可信隔离，尚未形成 OS 沙箱。
- FlowGuard 只做阶段产物预检，齐备仅 requires_approval，从不产生 allow；真实审批身份、当前测试/语义证据仍由完整治理流程处理。
- SpecGuard ALLOW 仅结构完整。快照是 untrusted_observation，不是已批准基线或需求实现证明。对本仓真实历史规格审计返回 INDETERMINATE：25 个来源、20 条识别需求、0 findings，bootstrap-guard-protocol 的自定义旧标题无法覆盖；未改历史规格掩盖结果。
- TestGuard 一致结果仅 consistent_untrusted，execution_verified 始终 false；真实执行器、Cargo 结果适配、来源和新鲜度尚未实现。
- ArchGuard 编译器类型推导/运行时调用/领域业务不变量等边界沿用 0.5.0，不因联合验证而扩大覆盖。

下一批优先级见 [专业能力路线](../../../docs/professional-roadmap.md)：真实测试执行与来源 → 需求/实现/测试追踪 → 受信阶段证据/审批 → 固定 Git 候选 → 架构有效模型/类型证据。各工具与 CodeGuard 的成熟度仍存在差距。
