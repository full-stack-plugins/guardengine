# GuardCore CLI

`guardcore` 是 EG-K11 的公共命令入口，复用 GuardEngine 的契约、候选、计划、执行、私有证据、OPA 和独立来源核验库。它不更改旧 `guardengine evaluate/verify` 或 CodeGuard CLI，不创建执行授权或质量放行。

## 命令

```text
guardcore doctor
guardcore contract validate --file /absolute/contract.json
guardcore contract compile
guardcore plan preview --selection /absolute/selection.json
guardcore plan register --selection /absolute/selection.json --operation plan-1
guardcore plan restore --selection /absolute/selection.json --operation plan-1
guardcore run --selection /absolute/selection.json --plan-operation plan-1 --operation run-1 --obligation OB-1
guardcore evidence show --operation run-1-stdout
guardcore evidence verify --operation run-1-stdout
guardcore evidence source-verify --selection /absolute/selection.json --plan-operation plan-1 --file /absolute/source-attestation.json
guardcore policy validate
guardcore policy evaluate --selection /absolute/selection.json --plan-operation plan-1 --operation policy-1 --facts /absolute/facts.json
```

Windows 使用同样参数和绝对本机路径。操作名必须符合内核标识约束，原件引用以真实返回的 `stdout.operation` 等字段为准，不能根据示例猜测。候选选择使用 `guard-subject::Selection` 闭合 JSON，例如 `{"kind":"commit","commit":"完整OID","base":"完整OID"}` 或 `{"kind":"worktree","base":"完整OID"}`。Index、merge candidate 同样交给既有候选库，不从当前工作树替代指定 Git 对象。

`doctor` 分别报告 CLI、Skills、初始化标识、语言/构建标识、平台、固定 Git 和验证器观察。目录标识只能证明存在，不能证明该规格体系有效；没有 Skills 提供者就报告 `not_inspected`。验证器必须实际完成固定版本探测、可执行文件摘要核对、输出精确匹配，才报告该探测成功；这不证明专业检查可覆盖项目。探测在空私有目录执行，使用独立最小环境，不在候选目录执行构建。

缺少宿主配置或引导失败时，`doctor` 仍报告实际 CLI 与平台事实；Skills、项目初始化、语言/构建、Git、验证器分别为 `not_inspected`，并附相同引导失败原因。此路径不读取候选、不启动版本探测、不创建临时目录，退出 3；不能把未检查的维度当成不可用或已通过。

`contract validate` 只做公共协议 schema 与语义校验，不获得批准。`compile` 从固定宿主引用读取三层原件及 source 字节，逐次调用真实签名/角色/范围/期限/撤销端口后生成 `CompiledContract`。普通 JSON 不能构造编译资格。

`plan preview` 绑定真实候选并保留完整义务，缺目标返回不完整。`register` 显式持久化冻结快照并注册 active head；已有任务不可通过新契约直接注册绕过迁移。`restore` 必须重新编译、重新采集和核对当前 head，不把任意计划 JSON 反序列化为 `RegisteredPlan`。本入口暂不提供批准迁移命令；迁移仍需既有受控 API 和独立批准。

`run` 仅执行当前注册计划指定的 domain obligation，使用 Runtime v2 私有物化目录、固定原生程序/参数/环境、共享绝对截止、前后输入核对及原始双流私有存储。重复 operation 不能静默重派发。它可报告 opaque 来源完成凭据是否产生，但不输出凭据、不持私钥、不自动签名。

`evidence show` 默认仅显示脱敏摘要；`--raw` 显式输出私有原件 base64，可能含敏感内容。`evidence verify` 只核本地原件完整性。`source-verify` 才消费外部独立签名来源原件，重新检查注册来源、真实计划头、run sequence、绑定、raw 引用、期限与当前撤销；它不从数据库重新铸造 Runtime 完成凭据，也不把来源签名当作专业质量结论。

`policy validate` 核包 manifest、文件与 capabilities 摘要。`evaluate` 使用当前契约和计划下的固定 OPA 版本、摘要、参数、环境和真实 Rego；undefined、异常、超时、截断不生成通过结果。流程事实属于 FlowGuard，CLI 不把事实 JSON 自报的批准变成受信授权。

## 宿主引导与配置

生产构建固定读取：Windows 系统 KnownFolder ProgramData 下的 `GuardCore/host.json`；Unix `/etc/guardcore/host.json`。不会使用 `ProgramData` 环境变量替代系统 KnownFolder，不接受 `--authority-config`、候选目录配置或运行时环境变量换根。默认入口不存在时返回不完整，不安装、初始化、索引或回退到较弱执行路径。

宿主部署者需要预先发布真实配置、独立签名批准、信任锚、固定工具与当前撤销文件。配置路径固定及内容 hash **不能证明 OS 保护**；本版本始终输出 `hostProtection: unverified`、`mode: cooperative` 和 `grant: null`。部署者须保证候选和调用方不能改写宿主配置、信任锚、工具及撤销发布源；需要操作系统强制保护的部分属于后续隔离/授权任务。

`host.json` 是闭合的 `guard.cli-host/v1` 对象：

| 字段 | 含义 |
| --- | --- |
| `project` | 已存在的绝对 Git 项目目录 |
| `repositoryId`, `taskId`, `actor`, `audience`, `worktreeId` | 受保护宿主明确指定的身份，不从 remote、目录名或候选 JSON 猜测 |
| `action`, `authorizationRefs` | 公共 action 枚举和独立授权引用 |
| `taskBinding` | 根内相对路径，内容必须是正式 TaskBinding 协议原件 |
| `layers` | 每项恰含 `level`（organization/project/task）、`document` 和 `sources`；document 是根内 ContractSnapshot 原件；sources 为 source contentDigest 到根内真实字节文件的映射 |
| `git` | 恰含 `executable`（绝对路径）、`artifactDigest` 和显式 `environment` |
| `verifiers` | 1 至 64 项固定验证器描述，恰一个 policy_engine |
| `policy` | 根内相对包目录，包含正式 `manifest.json` 及固定 Rego/capabilities 文件 |
| `budget` | 正式 ExecutionBudget：timeoutMs/maxOutputBytes/maxConcurrency |
| `environmentDigest` | 外部批准环境基线的声明摘要，不是本工具实测全 OS 的结论 |
| `stateRoot`, `scratchRoot` | 绝对私有目录；与项目/宿主目录、以及彼此必须互不包含；scratchRoot 预先存在，stateRoot 只由显式 register 创建 |
| `retentionMs` | 大于 0 且不超过 30 天的私有原件保留期 |
| `anchors`, `receipts`, `revocations` | 独立批准核验所需的当前文件端口 |
| `producers` | source-verify 使用的独立来源注册记录；不执行 source-verify 时可不配置 |

根内相对文件路径只接受普通 `/` 分段，拒绝空段、`.`、`..`、反斜杠、绝对路径、链接与 Windows reparse 路径。输入字节有界，JSON 拒绝重复键；读取前检查 regular，打开后再检查，Unix FIFO 不能作为普通输入。以上检查不宣称消除拥有同等 OS 权限的恶意替换竞态。

验证器每项使用闭合字段：`verifierId/version/artifactDigest/configDigest/adapterVersion/kind/owner/evidenceTypes/parameterSchema/executable/argv/environment/probeArgv/probeEnvironment/probeStdout/probeStderr`。kind 为 `domain` 或 `policy_engine`，owner 对应公共领域；configDigest 必须等于既有 `NativeVerifierCommandV2::configuration_digest` 或 `OpaConfig::configuration_digest(package)` 的真实公式。不是临时扫描工具后把新 hash 当作批准。domain 版本参数仅 `--version` 或 `-version`；OPA 为 `version`。probe 环境只支持 `SystemRoot/WINDIR/LANG/LC_ALL`，至少一条预期输出非空，精确匹配双流。

所有 domain 验证器必须使用同一已批准完整环境模板，显式包含：

```text
GIT_CEILING_DIRECTORIES={{subject_parent}}
GUARDCORE_GIT_PROVIDER_DIGEST=<实际固定Git描述的JCS摘要>
GUARDCORE_ENVIRONMENT_BASELINE_DIGEST=<host.environmentDigest>
```

Git 描述摘要公式是公共 `digest_json` 应用于 `{"schemaVersion":"guard.cli-git-provider/v1","platform":{"os":OS,"arch":ARCH},"provider":host.git}`。CLI 校验这些标记后，以完整显式环境模板的实际摘要绑定 capture；不偷偷注入、更改批准 pin，也不接受 Git 更新而继续沿用旧环境。argv/env 中的 `{{subject_root}}`、`{{subject_parent}}` 由 Runtime 控制渲染至实际私有目录，nonce 不改变批准 config。此身份覆盖固定主程序与显式配置，不证明 DLL、Git helpers、Java 依赖、外部文件或完整 OS 状态均被捕获；专业 adapter 仍负责其输入闭包。

GitProvider 在同步 main、创建 Tokio 线程前配置。所有真实 capture/verify 使用固定绝对 Git 主程序，清空继承环境并在每条命令前后核验 hash；不会靠 PATH 或调用者 cwd 中的 `git` 替代该身份。

批准文件端口：

- `receipts`：引用到 `{receipt: "相对路径", material: "相对路径"}`，分别为公共 ApprovalReceipt 与 guard-authority 的 ApprovalMaterial v1。
- `anchors`：每项恰含 `keyId/issuer/publicKey/revocationAuthority/identities/audiences/intents`。publicKey 是 32-byte Ed25519 公钥的标准 base64。identity 恰含 `principal/role/purposes/intents`，purpose 为 organization_layer/project_layer/task_layer/task/migration。角色权限与批准 scope 由真实 authority adapter 验证。
- `revocations`：receiptId 到根内当前文件；每条恰含 `authority/issuer/keyId/receiptId/revocationRef/revision/stateDigest/validFrom/expiresAt/revoked`，必须匹配请求完整 tuple。每次核验重读，不把缓存 bool 当当前状态。跨进程、跨重启撤销版本不可回退由受保护发布端保证；单个 CLI 进程的高水位不能替代该部署前提。
- `producers`：使用 ProducerRegistration 的明确字段 `issuer/keyId/publicKey/revocationAuthority/revocationRef/audience/repositoryId/taskId/contractDigest/policyDigest/scopeDigest/producer/domain/verifierPinDigest/profile/approvedInvocation/allowedSubjectKinds/maxValidityMs`。公钥同样标准 base64；完整 pin/profile/argv/env/候选种类必须独立匹配，不因当前计划 JSON 存在就信任来源。

每个端口先重新读取整个 host.json 并核对初始 JCS 摘要；进程执行中修改 actor、task、映射或锚列表将使当前操作失效。引用稳定的 receipt/revocation 原件仍每次重读。CLI 没有生成批准、注册 key、签名或全局安装子命令。

## 取消、退出码与副作用

`run` 与 `policy evaluate` 的执行阶段接收 Ctrl+C，并可显式增加 `--cancel-request ABS`。该路径必须是 scratchRoot 的直接子文件，内容为闭合 JSON：

```json
{"schemaVersion":"guard.cli-cancel/v1","repositoryId":"repo-id","taskId":"task-id","operationId":"run-id"}
```

精确匹配的已有文件在派发前同步取消；执行中每 25ms 有界观察。建议发布者在同一目录写临时普通文件后原子 rename。错误、越界、链接、超预算或不匹配请求会停止执行并明确返回输入错误，不会忽略后假称取消成功。监视任务随命令释放，取消保留已取得的原始输出。取消监听只覆盖实际 run/policy evaluate 执行阶段，不覆盖引导、批准编译或所有 doctor 探测；Windows 真实控制台 Ctrl+C 事件尚未本机实测，真实跨进程测试使用上述显式通道。

| 退出码 | 含义 |
| --- | --- |
| 0 | 请求的机械操作完成；不表示质量通过或获得 Grant |
| 1 | 明确拒绝，例如 schema 不合规、来源签名/当前资格拒绝、政策 DENY |
| 2 | CLI 或请求输入错误，包括坏 selection 与错误取消消息 |
| 3 | 缺宿主/批准/工具、目标不全、过期计划、重复 operation、超时、undefined、后置复核未完成等不完整状态 |
| 4 | 私有证据数据库/原件故障或宿主内部运行故障 |
| 130 | 执行阶段收到明确用户取消，观察与部分原件仍保留 |

`run` 的 0 还要求已完成后置核对、无漂移/截断、且终结实际 Applied；AuditOnly 或 recheck_error 即使进程 exit0 也返回 3。内核的 state=Incomplete 可仅表示尚待专业判定，不能一律把它当进程失败。所有报告固定 `quality: not_evaluated`，政策技术 ALLOW 与来源核验成功均不能签发 GateDecision/Grant。

| 操作 | 实际副作用 |
| --- | --- |
| help、contract validate、policy validate | 本地有界读取，不安装/初始化/联网 |
| doctor、contract compile | 空私有临时 cwd、固定版本探测进程；无候选构建 |
| plan preview | 上述探测、只读 Git plumbing 与受管私有临时 Git 数据；不写原 refs/index |
| plan register | 上述操作加私有 SQLite/原件/active head 写入 |
| plan restore、evidence、source-verify | 本地读取；打开 Store 可进行崩溃恢复/损坏隔离，source-verify 另核真实 Git/current |
| run | 候选私有物化、原生进程树、私有 stdout/stderr/metadata 与 ledger 写入；原项目不执行构建 |
| policy evaluate | 私有政策包/输入物化、固定 OPA 子进程、私有观察与 operation 记录 |

不会自动安装 CLI/Skills、初始化 SDD、索引、下载政策/工具或执行 Git fetch。副作用记录和错误诊断必须一起阅读，不能只看返回码。

## 构建与真实验收

普通 Cargo 构建生成 `guardcore`，不会安装到 PATH。`test-fixture` 非默认 feature 只为隔离验收：在**编译前**设置 `GUARDCORE_TEST_HOST_ROOT` 到测试专用普通目录，其值编译进二进制；生产构建不读取该变量。测试拒绝已有 host.json，根目录只能空或带明确测试 sentinel，并只清理本次自身原件和 TempDir。

真实测试额外需要 `GUARDCORE_TEST_GIT_EXE`（实际绝对 Git）及 `GUARD_POLICY_OPA_EXECUTABLE/GUARD_POLICY_OPA_SHA256`（经外部批准的固定官方 OPA 1.21.2）。不使用 PATH 找替代 OPA，不下载工具。CI 必须在实际步骤写 GITHUB_ENV 后再构建 feature，不能把运行时环境注入当生产 bootstrap。测试有自己的固定纯 capabilities 与 Rego，不依赖外部 FlowGuard 工作目录。

```text
cargo +1.85 test --manifest-path crates/guard-cli/Cargo.toml --locked
cargo +1.85 test --manifest-path crates/guard-cli/Cargo.toml --locked --features test-fixture --test real -- --test-threads=1
cargo +1.85 clippy --manifest-path crates/guard-cli/Cargo.toml --locked --all-targets --all-features -- -D warnings
cargo +1.85 fmt --manifest-path crates/guard-cli/Cargo.toml --check
```

真实验收使用独立 Ed25519 批准、实际 Git/SQLite、原生程序和官方 OPA，验证私有 Commit 字节与 dirty 工作树不同、原目录不构建、完整来源签名复验、撤销与伪 Git 拒绝、幂等不重派发以及取消/超时保留部分输出。Unix FIFO 负例由 Linux CI 实跑，本机 Windows 结果不能替代 Linux 实测。该候选实现是否完成 EG-K11 由正式任务账本与独立审查/CI 终态决定，本 README 不代替验收。
