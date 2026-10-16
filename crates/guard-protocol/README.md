# Engineering Guard interchange v1

状态：v1本地协议/字段清单已冻结并通过EG-K01局部验收，分发状态仍为candidate-unpublished，尚未公开发布；独立包 `guard-protocol`，不改变 GuardEngine v1alpha1/v1alpha2 或 `guard-evidence` 签名封装。

协议事实源为 `codeguard/docs/engineering-guard/protocol.md`，对应 `EG-K01` 与 `EG-EVI-004`。此 crate 负责闭合交换类型和分发资料，不实现契约编译器、规则执行器、信任注册表或授权签发控制端。

## 冻结对象

所有根文档有 `documentType` 判别字段和精确的 `schemaVersion: "1.0.0"`，专业域 owner 仅为 `specguard`、`archguard`、`codeguard`、`testguard`、`gitguard`、`flowguard`。根文档不能自报 `trustLevel`。

| 根文档 | 保留的核心语义 |
|---|---|
| ContractSnapshot | 来源、批准引用、规则、义务与契约版本 |
| RuleDefinition | owner、ENFORCE/REVIEW/ADVISE、严重性、适用条件、验证器、证据和例外策略 |
| TaskBinding | 需求、契约 digest、精确范围、session/worktree、既有授权、任务依赖 |
| SubjectSnapshot | worktree/index/commit/merge_candidate 的 Git wire，或独立 release_artifact 制品/来源/构建身份 |
| ProducerIdentity | 工具、适配器、配置、制品及执行环境身份声明 |
| VerificationPlan | 执行前冻结的候选/契约/策略、义务、精确目标集合、预算 |
| FactEnvelope | symbol/edge/sourceSpan、索引、解析方式、覆盖与限制；没有放行权 |
| GuardResult | 执行状态、事实结论和覆盖分离；保留超时前发现的违规 |
| GateDecision | 结果、批准、缺失义务、活动违规、原因、动作和有效期声明 |
| ApprovalReceipt | 范围批准/候选验收/操作授权分离、撤销引用、外部来源和签名声明 |
| ExecutionGrant | 精确动作、资源、受众、候选/目标身份、operationId、nonce 和外部签发证明声明 |

完整字段、必需性和枚举见 `model_metadata.json`、`schemas/engineering.schema.json` 与 `fixtures/valid/`。文件按对象分开；没有生产 wildcard import 或 stub。

## 批准范围与候选输入分为两阶段

`ContractSnapshot.obligations` 使用 `ContractObligation`：义务 ID、规则、验证器、必需标记和证据集合保持完整，`targets` 为 `TargetSelection {repositoryId, path, selector?, protectedDigest?}`。普通目标只批准范围，不把每个需求候选的源码摘要写入组织／项目契约；同一范围正常修改代码不会自行改变契约或要求重新批准原规则。

`VerificationPlan.obligations` 继续使用 `Obligation`／`ExactTarget`，在执行前把范围中的每个目标绑定实际 `inputDigest`。绑定不得减少规则、必需义务、证据或目标，也不得改 repository/path/selector。`TargetSelection::bind(actual_digest)` 保留这三项身份，并在 `protectedDigest` 存在时拒绝摘要漂移；未设置保护摘要时允许同范围绑定不同实际候选摘要。消费端仍须核验完整冻结集合、批准及实际候选，单项绑定不证明这些事实。

契约选择不接受执行计划的 `inputDigest` 字段；执行目标不能以 `protectedDigest` 取代必需 `inputDigest`。同一义务内相同 repository/path/selector 的重复选择，即使给出不同保护摘要，也会拒绝。受保护规则／测试等内容需要固定时显式设置保护摘要，不能静默重绑定。

这是尚未发布候选模型的修正，不是对已发布 v1 客户端的兼容承诺；原 GuardEngine alpha 和 `guard-evidence` 协议不变。

## 严格入口与权威边界

调用 `decode(&raw_bytes)`，不要先把不受信 JSON 反序列化为 `serde_json::Value` 再传入。入口在每一层拒绝重复键（包括转义后同名键）、未知字段/枚举/版本、非法无损路径、非法时间与矛盾的核心状态。输入有 2 MiB、64 层嵌套、8192 数组元素、2048 对象字段等界限。

`IngestedDocument` 保留原始字节，其 `trust_level()` 固定 `UNVERIFIED`。即使声明的 GateDecision 是 ALLOW，或 ApprovalReceipt/ExecutionGrant 的签名格式正确，也不代表签名、生产者、实际对象、批准、撤销、时效或操作权已经核实。此 crate 没有执行 API，也不会执行 remediation.argv。真实验签、受信注册表、当前对象核对与一次性授权消费属于后续独立控制端。

`encode` 再次检查公开类型字段的变更；核心字段不得通过扩展覆盖。扩展 key 使用 `namespace:name`，内容仅为未验证元数据。

## 规范字节与路径

规范字节采用完整 RFC 8785/JCS 数字规则：固定依赖 `serde_json_canonicalizer = 0.3.2`（Ryu-JS）与开启 `float_roundtrip` 的 serde_json，支持所有有限 IEEE754 binary64 数字、小数/指数输入、ECMAScript 指数阈值和负零规范化。对象键按 UTF-16 码元排序，标准 JSON 字符串转义，无空白，数组顺序不变。原始输入先逐层拒绝重复键、非法 Unicode 和非有限/溢出值，再进入规范化，避免被通用 Value 读者提前丢失重复键。

`canonicalize` 遵循 IEEE754/JCS 舍入语义，例如数值 `9007199254740993` 规范化为 `9007199254740992`，而字符串 `"9007199254740993"` 原样保留。这是 RFC 数值模型，不是任意精度十进制或金额表示。协议身份字段和 RunSequence 强制使用字符串；`decode` 另外拒绝超出安全整数范围的整数字面量，已定义的整型计数也有安全整数上界，防止候选身份依赖数值舍入。有限浮点规则参数可以正常使用。规范化能力和协议字段的更严格输入约束是两个层次。

`fixtures/rfc8785-appendix-b.json` 冻结全部 24 个有限 IEEE 位模式官方向量，覆盖正负零、最小次正规数、最大有限值、十进制/指数切换与舍入。`fixtures/canonical-vectors.json` 包含完整浮点样例、非 BMP 键序和字符串转义。`scripts/verify_jcs_ecmascript.mjs` 使用独立 Node/V8 `JSON.stringify` oracle 差分验证 4103 个确定性有限 IEEE 值、官方样例和 UTF-16 对象键序。不把 `guard-evidence` 原有字符串排序 JSON 摘要当作同一规范化协议。

RunSequence 的 Ord 使用整数顺序，wire 仍为规范十进制字符串。UtcTimestamp 不提供误导的字符串 Ord/PartialOrd；`compare_instant` 显式比较时刻，原文 Eq 独立保留（`.1Z` 与 `.10Z` 时刻相同但原始文本不同）。

参考：[RFC 8785](https://www.rfc-editor.org/rfc/rfc8785.html)、[上游实现](https://github.com/evik42/serde-json-canonicalizer)、[锁定版本 API](https://docs.rs/serde_json_canonicalizer/0.3.2/serde_json_canonicalizer/)。

GitPath 为 `{ "encoding": "base64", "bytes": "..." }`，保留原始 Git 路径字节，不做 Unicode 归一化、大小写折叠或有损 UTF-8 解码。拒绝绝对路径、NUL、空段、`.`、`..`；不把 repositoryId 等同本地路径。Git OID 长度与快照 objectFormat 交叉检查。

时间须为真实日历下的 UTC `YYYY-MM-DDTHH:MM:SS[.1..9 digits]Z`，不接受本地时区、闰秒或 year 0。合法时间字符串不证明受信时钟或尚未过期。

## 发布制品候选与 Git 候选分型

总体设计 D06 的第五种 `release_artifact` 使用独立闭合 wire；`documentType` 仍是 SubjectSnapshot。Rust `SubjectSnapshot` 是 `Git(GitSubjectSnapshot)` / `ReleaseArtifact(ReleaseArtifactSnapshot)` 的互斥联合，`as_git()`/`as_release_artifact()` 返回 Option，不隐式把制品当作 Git commit。原有四种 Git JSON 字段、kind、规范字节保持兼容；Rust 构造改为 `GitSubjectSnapshot { ... }.into()`。

release_artifact 必需字段：

| 字段 | 身份语义 |
|---|---|
| artifact | 受限逻辑 name、原始制品 contentDigest、精确 size、受限小写 mediaType；不接受本地路径/URI |
| source | objectFormat、完整来源 commit/tree、来源源码 inputManifestDigest；OID 长度与算法交叉核验 |
| build | buildId、definitionDigest、invocationDigest、全部构建 inputManifestDigest，以及固定 ProducerIdentity 声明 |
| inputManifestDigest | 本次制品验证的完整输入清单摘要，与源码/构建清单分开；不是制品字节摘要的别名 |
| schemaVersion/repositoryId/extensions | 固定协议、外部登记仓库身份和无升权扩展 |

size 为 0..9007199254740991 的整数；零字节制品是否适用由专业规则判断。ArtifactName 限制 ASCII basename，拒绝目录分隔符、点段、控制字符、尾点和 Windows 设备名；它只是逻辑标签，不授予写文件权。ArtifactMediaType 仅允许小写 type/subtype，不带参数、空白或头注入文本。release 分支拒绝顶层 worktreeId/objectFormat/base/head/tree/targetRef/expectedTargetOid/diffDigest；来源 Git 身份只能在 source 中明确声明。把 commit 的 kind 改成 release_artifact 不能通过。

构建身份的定义、调用、输入、构建器制品/配置/适配器/环境都固定为摘要声明；仍须独立核验真实来源、实际构建和批准。解析成功仍是 UNVERIFIED，不证明制品文件已存在、字节已被采集、来源 commit/tree 真有关联、构建可复现或发布已经执行。guard-subject/guard-plan 当前仅有 Git 采集/计划 provider，不能从这份协议 JSON 伪造 release capture。此补充也不表示 K20 的跨产品/provider/policy/toolchain 完整 release manifest 已实现或已经发布。

## Schema 与语义验证

`schemas/` 是 Draft 2020-12 闭合结构 Schema。`generate_schemas.py --check` 检查 Schema 与固定字段元数据一致。Schema 的 `x-semanticConstraint`/`x-inputConstraint` 标注无法仅靠结构 Schema 完整表达的校验（原始重复键、base64 解码后的路径、目标集合等价、字节预算、日历、字段值相等、时间顺序、UTF-8 字节长度与 Unicode 码点长度差异等），由 Rust 严格入口实施。

Identifier 的可表达结构约束已排除 ASCII/C1 控制字符与 Unicode 全空白字符串；UTF-8 字节预算明确标为语义约束，不能把 JSON Schema 的码点 maxLength 当作字节限制。

本包不包含通用 JSON Schema 求值引擎；本次仅添加普通 Cargo 规范化依赖，没有全局安装工具。**EG-K02 的通用 Schema 校验与契约编译尚未由此包完成**。测试同时覆盖 Schema 闭合与 Rust 入口，不能冒充独立第三方 Schema 引擎一致性认证。

## 兼容输入与分发

`project_legacy(bytes, "producer/version")` 明确保留旧 0.x 或 GuardEngine v1alpha1/v1alpha2 原件与 digest。返回始终 `UNVERIFIED`/`NOT_EVALUATED`，不从 `success`、`allow`、exit 0 或 `trusted:true` 推导通过；未知版本保守拒绝。这里只是无升权的审计容器，完整专业映射和差分迁移仍归 EG-K07。

`release-manifest.json` 固定 Schema、字段元数据与黄金 fixture 的摘要，状态为 `candidate-unpublished`；它不是密码学受信发布或完整六插件版本组合清单。

验证：

```text
cargo test --locked --offline --manifest-path crates/guard-protocol/Cargo.toml
cargo clippy --locked --offline --manifest-path crates/guard-protocol/Cargo.toml --all-targets -- -D warnings
python -B crates/guard-protocol/scripts/generate_schemas.py --check
cargo build --locked --offline --manifest-path crates/guard-protocol/Cargo.toml --example canonicalize
node crates/guard-protocol/scripts/verify_jcs_ecmascript.mjs <target>/debug/examples/canonicalize[.exe]
```

边界说明：冻结契约的来源批准、锁定规则、当前 Git 对象与候选真实性、原始证据存在性/摘要、独立签名与撤销、受信运行顺序、授权重放和服务端强制都需要后续消费控制端检查。协议声明本身不构成这些能力的验证结果。

本地冻结的逐项验收与112锁输入摘要见[EG-K01证据](../../docs/delivery/EG-K01-evidence.md)。本地冻结不替代K20完整产品/provider/policy/toolchain分发；wire变化须显式新修订或重新冻结并重新验收。
