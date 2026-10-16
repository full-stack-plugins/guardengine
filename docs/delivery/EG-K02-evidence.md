# EG-K02 契约编译局部验收证据

原任务：实现JSON Schema校验和契约编译，包括组织锁定、来源引用、冲突和验证器可用性；先以缺OPA/缺规则/冲突配置验证RED，再证明只读无安装/初始化。

| 任务义务 | 源码与可观察证据 | 结论 |
|---|---|---|
| 真Schema而非仅反序列化 | guard-contract/tests/schema.rs真实Draft2020-12组合/条件/引用/unevaluatedProperties；重复键、未知必需vocabulary/format、外部HTTP/file检索拒绝 | 满足 |
| 组织锁、来源、冲突 | tests/composition.rs组织规则不得被已批准下层覆盖；同ID不同语义冲突；来源原文摘要/缺失/重复校验 | 满足 |
| 验证器及只读能力 | 缺OPA/缺验证器/错pin/错owner/证据类型/参数Schema均阻断，保留规则；编译器仅使用显式内存材料和只读端口，无安装/init/fetch路径 | 满足 |
| 批准边界 | 缺独立端口、候选trusted字段/自行重算摘要、错任务/动作/编译身份不能得到CompiledContract | 满足 |

实际验证：23项Rust测试通过，另回放协议82个fixture：23有效、40原始JSON或Schema拒绝、19独立语义拒绝。作者与root分别完成Rust1.85回归、fmt及clippy；独立审查曾发现嵌入资源未知必需vocabulary被接受，已补行为RED并修复。两阶段目标测试先观察旧协议强制inputDigest失败，再验证ContractObligation选择范围和Plan实际摘要分离。来源参考tests/schema.rs、tests/composition.rs和本包README，不将模拟批准端口当生产信任根。

复验：`cargo +1.85 test --locked --offline --manifest-path crates/guard-contract/Cargo.toml`。

## 验证与边界

日期：2026-10-17。验收以原生 `codeguard/openspec/changes/add-engineering-guard-core/tasks.md` 的本任务字面为限；独立审查核对源码、真实行为反例与实际测试结果，未以任务勾选代替证明。Rust 1.85、固定 Cargo.lock、`--locked --offline`；TEMP/TMP、Cargo缓存与构建目标均在E盘。

EG-K01已按本地协议/字段清单冻结任务验收，仍保持candidate-unpublished；本库对未发布协议的局部实现已验证，不代表协议已经公开发布或P0产品集成可交付。CLI/MCP、真实受信授权提供者、OPA执行、有界runner、托管隔离与全链路验收仍由K06/K08/K09/K10/K11/K12及后续任务承担。不得据本任务关闭整条CON/EVI需求或P0路线；不sync/archive总体变更，不改既有alpha CLI或旧.codeguard状态。

后续协议release分型补充后，当前样本增至96个（25有效/71无效），contract全套23项再次回归通过。前述82个的Schema/语义分层计数是此前批次记录，不用推算代替新分层统计。
