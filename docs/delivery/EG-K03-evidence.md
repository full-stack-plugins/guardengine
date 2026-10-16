# EG-K03 冻结计划局部验收证据

原任务：实现TaskBinding、批准快照与冻结义务集合；验证候选自改规则、漏模块、运行中契约变化；批准快照不能由结果倒推。

| 任务义务 | 源码与可观察证据 | 结论 |
|---|---|---|
| 批准快照及全量义务 | guard-plan/src/planner.rs及tests/planning.rs由不可变CompiledContract生成全部义务；没有工具结果参数；缺目标保留全集和缺口，plan=None | 满足 |
| 精确输入绑定 | 真实Git verify_current拒绝伪造/陈旧CapturedSubject、错任务/工具pin；普通源码变化复用批准scope并刷新inputDigest，protectedDigest变化阻断 | 满足 |
| 唯一当前任务基线 | registration.rs以repo/task为稳定head key，session/worktree只存baseline，自报改归属不能另建头；冻结/JSON/load均仅draft | 满足 |
| 明确迁移与旧记录 | migration.rs外部批准绑定旧/新完整快照与精确revision；SQLite CAS唯一胜者，旧计划/assert/restore失效，原件与批准历史保留 | 满足 |

真实行为RED→GREEN：旧实现以worktreeId参与key，自报新worktree能绕迁移；实际Git/SQLite用例先失败，修复稳定repo/task头后通过。独立审查复核迁移等待期间竞争更新不能覆盖、同task初始并发绑定唯一胜者。外部批准端口显式注入，无默认自批。

实际验证：作者Rust1.85全套12项通过78.86s；独立审查使用独立goal-review-plan目标12项通过77.88s；root最终独立target root-plan、test-threads=2回归12/12通过，session89615退出0。fmt/clippy通过。实际用例全部位于tests/planning.rs，使用真实临时Git和SQLite，不是mock存储。

复验：`cargo +1.85 test --locked --offline --manifest-path crates/guard-plan/Cargo.toml -- --test-threads=2`。

唯一受保护registry与独立MigrationAuthority为调用边界；policy/environment摘要仅冻结声明，其真实性由K08/K06核验。RegisteredPlan证明当前绑定一致，不产生执行授权或工程PASS。

## 验证与边界

日期：2026-10-17。验收以原生 `codeguard/openspec/changes/add-engineering-guard-core/tasks.md` 的本任务字面为限；独立审查核对源码、真实行为反例与实际测试结果，未以任务勾选代替证明。Rust 1.85、固定 Cargo.lock、`--locked --offline`；TEMP/TMP、Cargo缓存与构建目标均在E盘。

EG-K01已按本地协议/字段清单冻结任务验收，仍保持candidate-unpublished；本库对未发布协议的局部实现已验证，不代表协议已经公开发布或P0产品集成可交付。CLI/MCP、真实受信授权提供者、OPA执行、有界runner、托管隔离与全链路验收仍由K06/K08/K09/K10/K11/K12及后续任务承担。不得据本任务关闭整条CON/EVI需求或P0路线；不sync/archive总体变更，不改既有alpha CLI或旧.codeguard状态。
