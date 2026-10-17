# guard-policy：固定 OPA 的通用策略执行端口

状态：候选实现，未发布。正式范围为 `add-engineering-guard-core` 的 EG-K08；生产流程组合规则属于 FlowGuard，位于其 `policy/engineering-gate/v1`。本包不拥有流程审批、例外和门禁规则，不签发 GateDecision、ExecutionGrant 或可信生产者证明。

```mermaid
flowchart LR
  C[已编译契约 / 注册计划] --> B[核对当前输入和 active head]
  P[闭合 manifest / 原件摘要 / pure capabilities] --> F[固定内存原件]
  B --> I[持久化单次消费意图 CAS]
  F --> I
  I --> W[私有策略包与输入副本]
  W --> V[固定原生 OPA version]
  V --> E[相同绝对截止下 eval]
  E --> R[分别保存原始 stdout / stderr]
  R --> A[复核副本及来源当前状态]
  A --> O[未授信策略观察 / 私有原件引用]
```

## 两种入口

- `OpaEvaluator` 是显式本地技术入口。调用者提供的 binding 和包身份是声明，不证明批准。闭合输入只接受 `guard.policy-input/v1`，领域 `facts` 保持不透明；FlowGuard负责其闭合模式和语义。
- `BoundPolicyEvaluator::prepare` 要求不可从任意 JSON 恢复的 `CompiledContract` 与 `RegisteredPlan`。核对真实当前 Git 候选、active head、契约摘要、plan.policyDigest、PolicyEngine 原件/版本/adapter/config pins 和预算上限；运行前后再次核对。批准与事实来源的独立真实性仍属于外部控制端及 K09，不能因为这次调用成功便升级信任。

策略求值不是专业义务执行，因此没有伪造 K06 RunScope 或另造通用执行账本。仅以 `guard-policy-operation:<operation SHA256>` 的 SQLite CAS 头单次消费操作号：意图持久化后即使 future 丢弃或进程崩溃，也不允许同 operation 自动重新执行。旧意图和结果保留审计；调用者明确选新 operation 才能重试。它不是授权或完整 RunLedger。

## 身份与执行边界

新增候选包身份规范：`PolicyPackage.digest = SHA256(JCS(完整闭合 manifest))`。manifest 包含 schemaVersion=1、packageId、version、owner、固定 data 查询、排序且唯一的文件路径与原始 SHA256、capabilities 路径与原始 SHA256。没有自引用 digest 字段。它是本次新增规则包身份，不冒称既有 ContractSnapshot 已定义同一公式。

只接受 `.rego` 或各层 `data.json`，最多64文件、每件1MiB、总4MiB；拒绝目录链接/reparse、路径穿越、Windows设备名、大小写别名、重复字段与错误摘要。读取后固定字节；候选自行改源包不能替换已加载的旧包。能力文件在私有 bundle 之外，必须显式空 allow_net，builtin 名称只能是当前纯函数闭合集；http/time/random/runtime不开放。OPA实际编译仍会拒绝未提供能力的调用。

`OpaConfig::configuration_digest` 使用协议JCS冻结 OPA 原件摘要、1.21.2版本、包/能力摘要、literal argv模板、显式清空后环境及全部预算。随机私有目录不改变批准身份；env禁止OPA_*覆盖和ASCII大小写重复。两个进程都复用 guard-runner（环境清空、原件前后摘要、Windows Job/Unix进程树终止、双流有界采集），版本探针与eval共享同一个绝对截止和执行器并发资源。版本为真实plain `version`，该版本不支持 `version --format=json`；原始Build Commit包含dirty字样也照实保存，不把版本文本当摘要真实性。

OPA入口建立同一个绝对截止，覆盖前置Bound来源核验、意图持久化耗时、私有副本准备排队与物化、runner排队、原件检查、版本探针和eval；不能在物化后重获整段预算。准备本身还受全局16槽及最多30秒约束，超时后worker仍持有permit及临时目录至实际退出，已经排队的blocking工作可能稍后收尾。准备耗尽记录TimedOut/无实际启动时间，保留空raw原件，不伪称来源漂移。同步prepare及结束后只读来源/副本复核分别有独立30秒有界期限；结果存储及结束复核不属于执行截止。future丢弃通过runner Drop收尾；不能宣称异步Drop已持久化完成状态。意图仍保留。

## 结果

OPA undefined、编译/执行错误、版本/摘要错误、取消、超时、输出截断、异常JSON、输入漂移均不形成可用结论。原始输入、manifest、能力、策略文件、探针及eval双流各自独立存为私有artifact，metadata只保留摘要和引用，既不base64塞日志也不编码成JSON字节数组。

输出闭合六字段：schemaVersion、decision、reasons、requiredActions、missingObligations、activeViolations。最后一项为有界领域对象数组，具体结构归专业策略；core不把对象转成字符串丢失阳性归属。`evaluation.incomplete` 表示执行/绑定机制未完整；成功求值的 `decision=INCOMPLETE` 是领域缺证据结论，两者不同。所有观察固定 `UNVERIFIED`；零退出码没有单独质量通过含义。

不是OS沙箱：同一操作系统身份的恶意并发替换、OPA本身漏洞、宿主DLL/OS及外部事实真实性没有由此解决。只读候选核验不意味着OS隔离。实际批准服务、证据签发、正规GuardResult→FlowGuard facts归一化、外部强制授权、完整性能基准仍各有后续职责。

## 验证方式

Rust MSRV 1.85。默认测试不要求安装OPA；真实测试须显式启用 `real-opa-tests`，并提供 `GUARD_POLICY_OPA_EXECUTABLE`、`GUARD_POLICY_OPA_SHA256`、`GUARD_POLICY_FLOWGUARD_PACKAGE`、`GUARD_POLICY_FLOWGUARD_CASES`。摘要来自独立批准的官方原件记录，测试不自动将现有文件摘要视为批准。

FlowGuard独立验收表用其 `tests/policy_gate.py --export-cases <path>` 导出，期望显式维护，core对27项有效信封真实执行比较，另1项无效信封由输入解析提前拒绝；合计28项组合差分。真实通用测试预算明确10秒，包含Git来源复核及运行期间漂移验证的绑定集成显式批准30秒；强制超时仍为6秒、准备排队耗尽仍为250毫秒。debug构建反复核验64MB原件，不能据此声称默认2秒预算或P0吞吐/分位数目标已经验证。实际终态结果由本次交付证据另列。

Linux真实CI曾因64MB OPA在未优化debug SHA后置核验耗尽固定两秒而拒绝：OPA实际7ms正常退出且版本正确，拒绝状态ExecutableChanged并不单独证明字节改变。dev/test仅优化sha2压缩计算，保留同一完整SHA256核验及全部截止、版本和pins；细节见VALIDATION.md，新CI成功前不宣称该修正已在Linux通过。
