# EG-A01 ArchGuard 独立字节码闭环验收

核验日期：2026-10-17。原任务：独立 ArchGuard 产品入口、ArchUnit 适配和最小批准分层规则；合法/禁止边/不完整输入真实验证，规则不写入公共内核。

实现 owner 为 ArchGuard，Java 专业规则和实际 ArchUnit 评估均位于该仓；GuardCore 只提供协议、批准契约、冻结计划、候选、证据和受限执行机制。首批最小规则是精确包前缀禁止依赖，不扩大成全部领域/对象/方法规范。

| 原任务范围 | 具体证据 |
|---|---|
| 独立入口 | 0.6.0 的 bytecode check/verify CLI、本地明确技术配置；现有 Java/Cargo、统一项目基线与需求增量不变；插件与注入提示说明新入口及未授信边界 |
| 真 ArchUnit | JDK17/default Maven 多模块实际编译，同一私有候选内导入本次 class 并调用 ArchRule.evaluate；合法、禁止边、源码行、缺件/损坏/空输入与编译失败均有实际测试 |
| 批准分层规则 | 规则从真实 CompiledContract 向指定 ArchGuard verifier 投影；未知/错owner/错参数拒绝；普通JSON不能创建批准对象 |
| 公共闭环 | RegisteredPlan/Runtime v2 绑定完整候选、工具/配置/环境、原始双流及当前序列；逐义务公共 GuardResult 保留 mode/severity、Private refs 与 UNVERIFIED，不生成授权 |
| 修复复验 | 本次 Commit 忽略dirty Worktree；Worktree真实违规；旧收据篡改与有效base漂移拒绝；修改源码后重新编译/verify成功，新subject与previous原件引用真实绑定；原仓无target |
| 原行为兼容 | root最终Rust1.85默认67项全通过，包含原Maven/Cargo、模型及Java源码闭环；严格Clippy、插件固定依赖闭包、安装及stdio包验证通过 |

实际TDD及修复过程见 ArchGuard `docs/verification-bytecode.md`：Partial 早退归属、候选外输入、Windows原生cwd、基线漂移与注入提示均先有真实失败，再修复复验。最终root在当前0.6.0和重新核验的275工具pin上独立两个真实E2E全部通过，325.13秒；Java19测试零跳过及14依赖锁通过。

发布实现提交 `86f0c2a3424214a715d78cce17fdc27425ef1292`，随后工作流修复提交 `2a237fc21b4848074e4b16e4d7212ef5b428f2b1`。前一CI因job级env错误使用runner上下文而未启动，不计通过；后者将目录配置移至实际步骤，未删测试或缩小接受条件。

- [ArchGuard CI 38987740407](https://github.com/full-stack-plugins/archguard/actions/runs/38987740407)：已终态success，Rust、Ubuntu Java bytecode、Windows Java bytecode三job全成功。两个Java作业各实际执行Java19测试、依赖锁及两个Rust到Java真实E2E；Windows E2E为2/2，140.25秒，无跳过。
- [安装 CI 38987740448](https://github.com/full-stack-plugins/archguard/actions/runs/38987740448)：已终态success，Ubuntu及Windows两个job全部成功，包含旧引擎、安装、技能依赖和真实stdio验证。

据原任务字面及上述独立证据，EG-A01可标verified。A02/K13–K25、全方法绑定、领域行为、模型自动同步、可信批准服务、受保护CI、真实宿主、四平台签名发布及全部生产路线仍未由本项完成。最小支持范围明确拒绝测试源码、注解处理器、profiles和自定义编译；这类缺口不按完整覆盖通过。
