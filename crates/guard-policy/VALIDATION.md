# EG-K08 候选验证记录

2026-10-17，Windows amd64、Rust 1.85、官方 OPA 1.21.2；未发布，任务是否关闭由主任务独立验收。本目录没有提交或推送，未修改任务账本。

## 真实失败与修复

1. 最初缺失公共解析接口的测试实际 RED；实现后 GREEN。真实 FlowGuard 联验进一步在 known-violation 报 `invalid type: map, expected a string`（16.89s）：通用 DTO 原误把领域违规对象当 ID。改为有界对象数组保留原阳性归属，领域 schema 仍由 FlowGuard 负责。
2. 准备队列真实 RED：Tokio 仅一个 blocking worker 被占3秒，策略250ms预算，旧实现物化后新建截止，测试4.45s失败。统一入口截止后目标测试3.13s通过；实际evaluate须小于2秒，版本及eval均无启动时间，状态TimedOut，无结论。3秒尾部等待仅验证遗留worker真实结束。
3. 异常输出缺Map字段真实 RED：`{"result":[{}]}` 导致索引panic。改为显式get错误，5种缺字段负例返回Err，不panic；同级审查代理使用D盘独立Rust probe重新链接实证5/5通过。

## 最终整套

实际命令（环境变量路径与摘要须明确提供，不从当前文件自动自批）：

```powershell
cargo +1.85 test --manifest-path crates/guard-policy/Cargo.toml --locked --offline --features real-opa-tests -- --test-threads=1 --nocapture
```

终态会话84481，exit0：

- `bound_real_opa` 1/1，44.57s：真实Git、编译契约、注册计划、SQLite、OPA；错pin拒绝；运行期间修改候选后撤销结论并保留已产生stdout；旧计划拒绝再次执行。测试的批准配置/计划预算明确30秒，不改变生产边界。
- `contracts` 2/2：undefined、未知字段、5种缺字段不panic。
- `package` 4/4：JCS包身份、原件固定、unsafe/remote capabilities、路径/查询/owner/未知字段、OPA环境覆盖与Windows大小写别名拒绝。
- `real_opa` 5/5，207.30s：真实undefined/严格builtin错误/http.send编译拒绝/语法错误与旧包DENY固定；真实已启动OPA超时、预取消、输出截断；future中断后意图仍在且重开SQLite不能同operation重跑；准备耗尽无spawn；FlowGuard最终生产包与独立显式28case表组合差分（27项有效信封实际进入OPA，比较decision、missing、保留阳性数；1项无效信封由输入解析提前拒绝）。

共12个Rust测试通过（没有把0个doc-test或内部case再算成额外Rust测试）。FlowGuard39个原生Rego测试由其独立责任方验证，不混入此12的计数。

MSRV 1.85 `cargo fmt --check` PASS；`cargo clippy --locked --offline --all-targets --all-features -- -D warnings` PASS。Clippy最后只移除测试配置中的无意义借用，未变生产逻辑。

## 被保留的失败记录

较早整套40465中的timeout组合测试先通过实际TimedOut断言，随后预取消存证遭E盘满导致SQLite `database or disk is full`。恢复空间后仅重验该项，20748 exit0、14.15s。没有把该较早整套说成通过。

统一截止后的15341另在绑定测试10秒预算耗尽时正确NotRun，但测试错误地假定必有stdout而失败。用于验证真实已产出后撤销的绑定集成改为明确批准30秒测试预算后，最终84481如上通过；强制超时仍6秒、准备预算仍250ms、通用测试仍10秒。没有放宽生产截止或宣称每个Timeout必有部分输出。

## 未被此记录证明的事项

- 无本机Linux运行证明；跨平台由CI独立验收。
- 未测默认2秒性能目标或30次分位数样本。
- K09事实/批准权威、K10隔离、L02正规GuardResult到领域facts归一化、签发Grant没有由本包实现。
- 测试ApprovalAuthority和facts为显式fixture，只证明批准接口绑定，不能作为生产授权服务。

OPA语法与CLI依据：[官方CLI](https://www.openpolicyagent.org/docs/cli)、[策略语言](https://www.openpolicyagent.org/docs/policy-language)、[测试文档](https://www.openpolicyagent.org/docs/policy-testing)。实际1.21.2 plain版本及命令行为另经真实执行核对。

## 主任务独立复验

终态会话94676，exit0：Rust1.85全套12/12通过，绑定真实OPA测试48.61s、contracts 2/2、package 4/4、真实OPA组5/5 211.79s；包括27项OPA求值及1项无效信封提前拒绝的28项组合差分。使用现有官方E盘OPA，临时目录在D盘。共享CI准备脚本另15/15通过（0.307s）。首次独立调用把摘要环境变量误写成裸hex，因Digest格式非法而失败；改为批准的sha256:完整值后执行上述整套，没有修改生产校验。跨平台CI尚待发布后独立确认，EG-K08未勾选。
