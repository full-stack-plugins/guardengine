# Guard Contract：契约编译候选内核

本独立 Rust 包落实 `add-engineering-guard-core` 的 EG-K02，规格事实源为 CodeGuard 的 `engineering-contracts/spec.md`、`docs/engineering-guard/protocol.md` 和 `technology.md`。它复用相邻 `guard-protocol` 的冻结协议，不改变现有 v1alpha 行为，不提供 CLI/MCP，也不宣称整个 Engineering Guard 已完成。

## 输入、组合与身份

调用方通过内存显式提供组织、项目、任务三个层次的原始契约、来源原始字节、任务上下文、拟执行动作、验证器描述及只读端口。三个层次必须各出现一次；重复层次、缺失层次、空规则均产生 `Blocked`。同 ID 且完全相同的规则和义务可以去重；组织规则全部锁定，下层不得覆盖，即使下层自身已有批准。项目与任务同 ID 不同定义也明确冲突。义务不能通过覆盖目标、删除证据要求或更换验证器而缩减。

每份来源原始字节必须与声明的完整 `SourceRef` 一一对应，实际 SHA-256 必须等于 `contentDigest`；遗漏、重复、多余材料及字节篡改都被拒绝。编译器不读取引用路径，也不联网获取来源。单份材料上限 4 MiB、每层总材料上限 32 MiB、最多 8192 份；输入层次超过八个直接阻断，诊断最多 256 条且单条详情截断为 1024 字符。

`snapshot_digest` 定义本编译器使用的契约身份：将完整 `ContractSnapshot` 按协议封装序列化，仅删除自引用的 `digest` 字段，然后以 RFC8785 JCS 规范化并计算 SHA-256。其余字段（包括 approvalRef、sourceRefs、版本）都参与摘要。调用者必须使用这一明确算法准备批准记录，不能把任意自填 digest 当作批准证明。

成功产物包含来源引用、逐层批准核验记录、规则与义务、实际 Schema 摘要、任务/动作及精确验证器身份。层次、规则和验证器按固定次序组合，输入层次重排不改变编译身份。最终编译摘要还绑定外部逐层批准者身份。产物字段私有、只提供借用读取、不实现 Deserialize；候选 JSON 无法直接构造一个已批准编译结果。修改注册的验证器身份会改变编译摘要，必须由既有任务授权重新核验。

契约义务使用 `ContractObligation` / `TargetSelection`，保存经批准的 repositoryId、path、可选 selector 和可选 protectedDigest，不将尚未执行的候选代码 inputDigest 写入契约。普通候选字节变化交由后续计划绑定，不要求重新批准相同目标范围。修改目标范围、selector、保护摘要或其他契约字段会改变契约摘要，必须重新获得外部批准。已批准的 protectedDigest 原样保存，后续计划绑定不能漂移；此编译器不读取候选字节，实际摘要核对由计划模块负责。运行阶段仍使用独立的 `Obligation` / `ExactTarget`，不将两种类型混用。当前义务目标只接受与授权上下文相同的单个目标仓库；来源可以来自独立策略仓库。跨目标仓库授权与完整目标集合冻结不在本包实现范围。

## 真正的 Draft2020-12 求值

使用固定 `jsonschema = 0.33.0`，不是仅依靠 serde 字段反序列化。支持实际 `$defs`/本地引用、组合、条件、数值约束、unevaluatedProperties 等求值；先严格解析原始字节，保留重复 JSON key 拒绝能力，再执行真实 Schema 和独立协议语义检查。

仅接受 Draft2020-12，标准子 Schema 位置内的嵌入资源也检查其 draft 和必需 vocabulary。未知且 required=true 的 vocabulary 明确失败。主动启用 format 断言并拒绝未知 format，比默认 format 注解模式更保守。`const`、`default`、`examples` 等数据中的 `$ref` 或 `$vocabulary` 不作为 Schema 指令扫描。

依赖关闭全部默认 feature，HTTP/file resolver 不启用；所有实际外部资源检索统一由 `DenyExternal` 拒绝。已有本地引用可解析，外部 HTTP/file 引用阻断；编译期间不访问网络或文件。协议 Schema 是构建期 include 的相邻冻结文件，其原始字节摘要进入编译产物。Schema 的结构通过不等于协议语义或可信批准通过：路径、字节长度、时间和跨字段约束仍由 guard-protocol 校验。

依赖官方说明：[jsonschema 0.33.0 文档](https://docs.rs/jsonschema/0.33.0/jsonschema/)、[官方仓库](https://github.com/Stranger6667/jsonschema)。下载到Cargo缓存的实际0.33.0 manifest声明MSRV 1.71.1、MIT；本包声明Rust 1.85并提交独立Cargo.lock。已用实际Rust 1.85独立运行全部23项测试，不只依赖manifest或稳定工具链推断MSRV兼容。

## 受信端口与能力端口

`ApprovalAuthority` 是宿主显式注入的只读信任边界：

- `verify_layer` 核验既有逐层批准。返回的契约摘要、批准引用、层次、任务上下文和动作必须与请求完全相符。
- `authorize_task` 在最终编译身份确定后，核验既有任务授权。返回引用必须属于请求已声明的 authorizationRefs，且绑定精确任务、仓库、动作和编译摘要。

缺少端口或批准、引用不符、外部拒绝均阻断。候选数据不能自带 `trusted:true`、签名样子的数据或重新计算的摘要来绕过端口。本包不创建批准、不签发 ExecutionGrant，也不实现密码学验签、身份认证、有效期、撤销或持久化批准存储；这些必须由独立控制的宿主/后续可信控制器实现。普通 Rust 调用者能够自行实现 trait，因此这里实现的是明确可审计的集成边界，并非隔离候选进程的安全沙箱。不能将测试里的模拟批准实现用于生产信任根。

`VerifierRegistry` 固定验证器 ID、版本字面量、制品摘要、配置摘要、适配器版本、参数 Schema 摘要、专业归属和可产出的证据类型；拒绝浮动版本/范围及同 ID 不同身份重复注册。版本身份由精确字面量和摘要固定，不声称执行 SemVer 合法性检查。参数经真实 Schema 求值，专业 owner、规则指定验证器及义务证据类型均须匹配。

`CapabilityProbe` 仅接受宿主对已存在能力的只读观察。可用观察必须与全部 pin 完全一致；Missing、Unknown、Unsupported 或身份不符都阻断。策略引擎必须独立注册并可用，缺少 OPA 或专业验证器时保留规则与义务、输出明确诊断，绝不以空规则替代检查。此包不运行 OPA 或任何验证器，不下载/安装 SDK，不执行 init/fetch，也不扫描 Git。trait 无法阻止宿主实现自行产生副作用，宿主必须遵守只读约定；单元测试不证明真实 SDK 或 OPA 的安装资格。

`ReadyForPlanning` 仅表示这些编译前置条件满足，不是 Gate ALLOW、工具执行许可或完整 VerificationPlan。真正的计划、OPA 求值、执行控制器与授权仍须由后续模块实现。

## 验证证据与剩余范围

23个测试覆盖真实Schema正负例、未知vocabulary/format、嵌入资源、禁止外部检索、重复键、来源篡改、组织锁、规则冲突、伪造自批、跨任务批准、缺OPA/验证器、错误pin、参数约束和冻结授权。另回放guard-protocol全部82个黄金用例：23份有效、40份由严格JSON/真实Schema拒绝、19份由独立协议语义拒绝；没有将三种证据混为同一层。端口模拟只验证集成约束，不代表真实人类批准或生产验签。

真实 TDD 记录：先建立缺少 API 的失败验收；随后实际行为反例暴露了对 const/examples 中 `$ref` 的误判、加入批准来源后输入重排导致摘要不稳定、以及 jsonschema 对嵌入资源未知必需 vocabulary 的静默接受。两阶段目标调整另先观察旧协议强制 inputDigest 导致已批准无保护目标范围被拒绝，再适配契约选择与运行绑定的独立类型。均先观察测试失败，再修复并复验，不通过放宽负例获得通过。

复验命令（TEMP/TMP、Cargo 缓存和构建目标均配置到 E 盘）：

```powershell
cargo fmt --manifest-path crates/guard-contract/Cargo.toml -- --check
cargo test --locked --offline --manifest-path crates/guard-contract/Cargo.toml -- --nocapture
cargo clippy --locked --offline --manifest-path crates/guard-contract/Cargo.toml --all-targets -- -D warnings
```

本包已按原任务字面完成 EG-K02 的局部实现与独立审查，见[验收证据](../../docs/delivery/EG-K02-evidence.md)，不关闭整个CON需求。EG-CON003 的计划冻结与EG-CON004的显式迁移由guard-plan承担；本包不负责批准历史归档。EG-CON005 的 CLI/Skills 探测和项目发现仍由后续集成任务承担。Git subject、持久化证据、OPA 执行、真实信任根、执行授权、CLI/MCP 接入及端到端集成均需要后续模块。未改总体 tasks 或 CI。
