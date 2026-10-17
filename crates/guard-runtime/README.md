# GuardRuntime：已注册计划的原生执行衔接

本库把 `RegisteredPlan`、`CompiledContract`、`GuardRunner`、`GuardRunLedger` 和受管 `GuardStore` 接为一个本地机械闭环。它不是 CLI、真实领域适配器、可信授权服务或 OS 沙箱。退出 0 只表示进程完成，仍以 `Incomplete` 等待专业适配器解析，绝不自动产生质量 PASS。

显式 v1 档位仅允许 Worktree 候选在对应原项目目录执行。Commit/Index/MergeCandidate 必须选择 v2：四种 Git 候选在 v2 中都从已注册捕获原件物化到私有目录，执行器读取候选自己的字节，不能把 dirty 工作树的结果挂到另一个 Git 对象身份。发布制品执行提供器仍未实现。

```mermaid
flowchart LR
    A[当前注册计划与不可变契约] --> B[异步重采集当前Git]
    B --> C[精确义务与工具配置pin]
    C --> D[SQLite取得运行序号]
    D --> E{Started或Existing}
    E -->|Existing| F[返回原序号 不派发]
    E -->|Started| G[共享槽位与统一绝对截止]
    G --> P{显式配置档位}
    P -->|v1仅Worktree| W[原仓cwd兼容执行]
    P -->|v2四种Git候选| M[从冻结输入物化私有目录]
    M --> V[核对来源当前身份及物化输入]
    V --> R[受控渲染argv与env 私有cwd]
    W --> H[受管原生进程与实际UTC观察]
    R --> H
    H --> I[结束核对来源当前身份及物化输入]
    I --> J[双流独立私有原件与调用receipt]
    J --> K[小metadata与CAS终结]
    K --> L[执行观察 等待专业解析]
```

## API与绑定

`Runtime::new(&RegisteredPlan, &CompiledContract, Arc<EvidenceStore>, project, retain_until_ms)` 是显式同步准备入口，会运行实际 Git 检查；异步宿主应在自己的阻塞准备线程中调用。它拥有计划、契约和存储引用，并校验编译契约摘要与全部 verifier pins。一个计划复用同一个 Runtime 或其 clone，二者共享并发槽位和执行器；宿主不能通过为每项义务另建 Runtime 或另选空数据库绕过控制端资源与注册表策略。

`execute(obligation_id, operation_id, NativeVerifierCommand, Cancellation)` 不接受调用方 plan JSON、运行序号或 shell 命令串。义务必须在冻结全集中，验证器 ID 必须精确对应。原生 exe 摘要等于编译 `artifact_digest`，执行器前后实际读取文件校验。`NativeVerifierCommand` 提供公开公式：

- `configuration_digest` = JCS/SHA256 `{schemaVersion:"guard.native-command/v1", args, directory, environment}`。目录采用原始可表示 Unicode 路径字符串；调用方应先规范化目录，再据此编译批准 pin。
- `environment_digest` = 显式环境映射的 JCS/SHA256；必须等于 RegisteredPlan 只读 `capture_binding()` 的冻结声明。

v2 使用 `Runtime::new_v2(..., scratch_parent)` 和 `execute_v2(..., NativeVerifierCommandV2, ...)`，不会静默升级 v1 配置。scratch_parent 必须显式提供绝对已存在目录，物化层拒绝链接/重解析、原仓及 Git 控制目录；创建私有 TempDir 是 Started 后才发生的副作用，不修改原仓 refs/index/objects。物化对象所有权覆盖运行和结束核验，结束后由 TempDir 尽力清理临时目录；后台准备/复核仍持有对象时延后清理，异常占用或控制进程崩溃时须由宿主管理残留目录。

`NativeVerifierCommandV2::configuration_digest` 固定 JCS `{schemaVersion:"guard.native-command/v2", workingDirectory:{kind:"subject_root"}, argvTemplate, environmentTemplate}`。CommandSpec.directory 仍须指向逻辑原仓根，批准 pin 不包含随机私有目录。只支持逐参数、逐环境值的一次性 `{{subject_root}}` 与 `{{subject_parent}}` 受控替换，不递归解释生成路径中的花括号，不经过 shell，未知 token 拒绝。root 是私有 `TempDir/inputs`，parent 只取该 inputs 的直接私有 TempDir 父目录；不能上移到 scratch_parent。普通相对参数从候选目录读取，显式批准的外部绝对参数保持原样。

v2 的 environment_digest 固定原始模板，必须显式包含 `GIT_CEILING_DIRECTORIES="{{subject_parent}}"`，过时的 root-ceiling 配置直接拒绝。Git 对等于当前 cwd 的 ceiling 不阻止向祖先发现，因此不能静默把 root 模板暗换成 parent；批准 pin 必须真实包含正确边界。该语义由 [Git 官方文档](https://git-scm.com/docs/git#Documentation/git.txt-codeGITCEILINGDIRECTORIEScode) 和真实阳性/阴性对照验证。其他 `GIT_*` 覆盖、非 ASCII 合法变量名和 ASCII 大小写别名均拒绝。

控制端生成的 Windows verbatim drive/UNC 根转换为 Git/Java 可使用的等价原生表示，拒绝设备命名空间、相对根和会产生尾点/尾空格等别名的组件；Unix 绝对根保持原样。转换不作用于 GitPath。渲染后的实际环境、参数和 cwd 另计算 effectiveInvocationDigest；本次 receipt 同时绑定批准 configDigest、subjectDigest、inputManifestDigest 和物化目录。nonce 改变 receipt，不改变批准配置、计划或任务范围。Git ceiling 防止工具意外向私有目录的祖先发现别的仓库；程序自行重写环境、改目录或读取外部文件仍不属于 OS 沙箱保证。

绑定身份包含 TaskBinding、planDigest、完整 active head identity、subject/contract/policy/environment digest 及全部编译 verifier pins。运行 scope 绑定 repo/task/obligation 与批准 TaskScope 摘要，序号完全由持久账本分配。参数、配置和环境不会从候选报告中倒推。实际 DLL/JAR、OS、外部工具链与政策内容真实性仍由后续专门机制核验。

`Started` 是唯一派发入口；`Existing` 只返回序号，不再次运行。完成后的 operation ID 不能重启另一轮，调用方应使用新 ID。旧候选、旧 active head、错误义务或 pin 均拒绝；取得执行槽位后再检查输入，防止排队期间已知漂移仍启动。运行后的输入核验失败使结果不完整，已采集违规字节仍保存。

v2 在同一冻结截止内完成物化和派发前核验，比较只读捕获原件完全一致并重核原 source-current 与私有输入清单；执行后两者再次复核。私有输入被改写与原仓候选漂移都会标为 Incomplete，保留已观察原始输出；新增构建输出不冒充原始冻结输入。receipt 证明准备关联，不替代专业适配器把其参数/覆盖范围对应到冻结 targets。

## 原件、失败与恢复

stdout/stderr 分别以原始二进制 `store.put` 保存，各自引用含 operation、实际内容摘要和尺寸；不把大日志编码为 JSON 字节数组或 base64。账本只接收约数 KiB 的身份和执行维度 metadata。原始字节可能敏感，只通过私有存储读取；引用不代表字节通过领域规则校验。

非零为 Fail，超时/取消保留对应运行状态，输出截断或输入复核失败为 Incomplete。正常退出 0 也为 Incomplete。它们全部为 `local_unverified`，不签发回执，不授信、不推进交付授权。专业适配器应解析部分输出中的真实违规并明确覆盖不足，不能把不完整结果改写为完整通过。

startedAt/finishedAt 原样透传执行器实际进程创建与终止/排空后的 UTC 观察；未派发保持 None，不用 Runtime 的准备时间补造。墙钟无效/倒退保持失败，运行序号仍由 SQLite 单独分配，单调截止与墙钟观察不混用。

调用 future 被丢弃时，执行器 Drop 清理受管进程树；本库没有 async Drop 写库承诺。已 Started 的账本可能保留 Running，原件写入中断可能留下非 active 完整原件或 pending 意图，按 GuardStore/RunLedger 的显式恢复流程审计。恢复前控制端必须独立确认旧执行树已终止或隔离；`RunLedger::recover` 本身不证明进程死亡。CAS 迟到终结返回 AuditOnly，不能恢复旧运行资格。

原件存储或配额失败返回错误，不声称全部输出已经持久化；可能只有一个流或部分完整原件保存，运行头留待显式恢复。调用方必须处理该失败，不能把收到错误改写为无违规。

`execute` 内的 Git 重采集全部放在 `spawn_blocking`，有全局 16 个检查槽位。第一次只读准备检查独立最多 30 秒。取得运行票后计算一次冻结 `timeout_ms` 截止，Runtime 槽位排队、派发前重采集和原生进程共同消耗剩余时间，`run_until` 不重新给满额进程时间。派发前检查耗尽截止时保存 TimedOut 观察，不谎称输入漂移；没有进程就保存两个空原件。执行后的复核另有 30 秒，执行器终止/排空清理仍按 GuardRunner 的额外有界清理规则进行。

检查超时/取消后后台线程继续持有槽位直到真实退出，避免无限堆积后台 Git 检查；原生 OS 文件系统调用仍可能不可强制中断。复核超时/失败不会删除已有部分字节，也不能获得完整质量结论。同步 `new` 属于宿主独立准备阶段。SQLite/原件操作保留 Store 的有限 busy 等待，不提供远程数据库或可信墙钟证明。

## 验证与未完成范围

测试使用本包自己的 feature `test-fixture` 原生子进程、真实临时 Git 与真实 SQLite，不跨包引用私有测试代码。已观察到排队漂移反例：旧实现实际启动两次（marker 为 `xx`），目标要求只启动第一次（`x`）；新增共享槽位后必须再采集输入，拒绝第二次派发。

```powershell
cargo +1.85 test --locked --offline --features test-fixture
cargo +1.85 clippy --locked --offline --features test-fixture --all-targets -- -D warnings
cargo +1.85 fmt --check
```

真实 Git/Schema 跨测试负载以测试专用 RAII 许可限制为 2；每个用例内部的并发、单飞、取消和排队仍真实执行，生产检查槽位与冻结截止不因此改变。最初 12 个重型用例同时运行导致派发前预算耗尽时，记录确实为无 PID 的 Timeout/空输出；没有把它们冒充已产出的部分观察，也未放宽生产 deadline。

首批七项真实 E2E 在 Rust 1.85、最终 Windows runner 上完整通过（116.20 秒），包括超过 2 MiB 的双流私有原件、退出零不授质量结论、取消后真实前缀、排队漂移不派发、错误 pin/配置/环境/旧输入拒绝、future Drop 后 Existing 不派发及显式恢复递增序号、1ms 预算超时不伪称漂移，以及非零/超时/截断/运行中漂移。fixture 先 flush 前缀再写启动握手；超时未启动时允许真实空观察，已写出前缀则必须保留。

独立审查又发现 Commit 对象正确但执行目录为 dirty 工作树的输入绑定缺口。新增真实 Commit/dirty 反例在旧协调器失败（16.18 秒），加入明确非 Worktree 拒绝后同例通过（24.82 秒）。这是中间保护的新增 focused 证据，不把前七项结果改写为最终八项全套通过，也不证明四种候选物化已经完成。严格 Clippy 与格式检查通过。

后续 v2 接入真实物化：默认整套的 12 项 E2E 有 11 项通过、1 项 Git ceiling 对照失败（358.81 秒）。该反例证明 ceiling 等于当前 cwd 时仍会发现父仓；仅消除 Windows verbatim 前缀后仍失败，未把路径形式变化冒充修复。按正式语义将批准模板明确改为直接私有 parent，旧 root-ceiling 模板拒绝。

最终同一源码的受影响复验：`cargo +1.85 test --locked --offline --features test-fixture v2_ -- --nocapture` 四项 E2E 全通过（174.02 秒，其他八项过滤），包含四种 Git 候选实际读取正确私有内容、原仓不可读取的合并新 blob、nonce 不改变批准配置/计划、Git 发现阴性和解除 ceiling 后阳性、来源变化与私有输入改写后保留输出。`--lib` 四项单元全部通过，在 Windows 实际覆盖 drive/UNC 转换、设备/尾点别名拒绝、ASCII 环境闭合与模板一次性替换；Unix 原样路径分支未在本机实际执行。最终格式检查与严格 Clippy 通过。这里保留分阶段的真实结果，不声明单一命令已完成当前十六项全套运行。

这些测试没有证明生产批准服务、K09/K10、真实 Java/Maven/Cargo 专业适配、完整输入/依赖环境真实性、Unix 执行结果或整项 K06 已完成。本库不自动勾任务、不写旧 `.codeguard`、不安装工具，也不发布产品。

## K09附属来源凭据（本地候选，未发布）

新`execute_with_receipt`/`execute_v2_with_receipt`与旧API共享真实执行路径；旧API不执行新资格读取、不改变观察wire/trust。包装的`into_parts()`返回原观察和可选`DispatchCompletion`，`qualification_error()`保留附属采集失败；没有凭据不等于空结果或无违规。

完成凭据仅在真实ProcessRunner进程创建信号与finalize Applied之后内部铸造，绑定原始已保存metadata字节、实际双流、完整运行头和计划。Existing、未创建进程、AuditOnly没有凭据；公开ExecutionObservation、RunLedger.start/finalize或Store JSON无法构造它。真实进程创建不证明Windows已Resume，状态/时间仍原样保留。非零/超时/截断只证明相应部分字节的来源，零退出仍不证明规则通过。

`DispatchCompletion::verify_current`必须重读实际计划/候选、完整运行头、metadata与双流；新头或损坏使旧对象失效。跨重启不提供from_store恢复；只有独立受保护签名原件经authority复核后能取得独立来源对象。可解析`DispatchProvenance`及其只读存储检查本身不构成资格。env/argv在私有来源原件内，不自动公开。

真实反例已证明：公开账本能手造active PASS元数据；另一个调用者能取得Existing票提前终结。真实Runtime在后一种竞争中只能AuditOnly，不能铸造签名资格。来源签名与专业解析/领域归一化分离，生产隔离与grant仍属后续范围。

此来源profile只覆盖Runtime执行的domain verifier（编译pin具有专业owner）。policy-engine的owner=None不产生完成凭据；OPA继续由独立BoundPolicyEvaluator执行和核验，本批不重构K08，也不声称建成所有生产者/角色的统一签名体系。其他provider不能借本profile的opaque类型自行授信。
