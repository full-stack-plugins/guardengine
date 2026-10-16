# EG-K01 本地协议冻结验收证据

日期：2026-10-17。原任务：冻结协议v1和跨仓字段清单；新增protocol crate、闭合schemas及正负黄金fixture，验证未知版本/重复字段拒绝与旧报告保留。本任务为本地协议冻结，不是K20完整分发或实际生产发布。

| 原任务义务 | 实际证据 | 结论 |
|---|---|---|
| 协议v1和跨仓字段清单 | codeguard/docs/engineering-guard/protocol.md逐对象字段表；guard-protocol/model_metadata.json逐字段/枚举/required元数据与11根交换对象一致 | 满足 |
| 独立crate与闭合schemas | schemas/engineering.schema.json及根schema，精确schemaVersion/documentType，未知字段/枚举/版本拒绝；独立generate_schemas.py --check通过 | 满足 |
| 正负黄金输入 | 96个样本：25有效、71无效；包括11根未知字段反例、未知major、嵌套/转义后重复键、两阶段target混用和release伪装 | 满足 |
| 规范字节与冻结清单 | release-manifest.json锁112个输入，恰覆盖全部schemas/fixtures JSON及model_metadata，无遗漏/多项；独立逐文件重算SHA256且检查LF均通过 | 满足 |
| 原报告保留 | project_legacy保留原始字节与摘要、显式producer/version适配器；始终UNVERIFIED/NOT_EVALUATED，allow/success/trusted不能升级 | 满足 |
| 完整候选类型 | D06四种Git wire与独立release_artifact联合；制品/来源/构建/验证输入身份分开，不能把Git改标签伪装发布制品 | 满足 |

## 冻结点和实际验证

本地冻结点：protocolId `guard.partme.ai/engineering/v1`，schemaVersion `1.0.0`，manifest状态保持 `candidate-unpublished`。当前release-manifest.json原始字节SHA256为：

`sha256:dda8275e863e9c6f9cd3bb0a96dd2c88e523b6430e22c16cd318fb498af9f3a8`

这是本次本地冻结证据，不是密码学受信发布。后续wire/枚举/规范化/锁输入变化必须显式新修订或重新冻结并重新验收，不得保留旧摘要追认新内容。112锁输入严格LF，避免Git检出改变原始摘要。

Rust1.85协议27项测试通过，fmt/clippy/schema通过；96个正负fixture覆盖新release分型。JCS另有24个RFC8785有限数值向量、独立Node/V8 oracle的4103个有限IEEE值与UTF16键序差分证据。既有分发测试逐一核验锁输入摘要与LF；独立审查另重新枚举实际112输入确认完整性。release新增后受影响contract23/subject9/plan12均回归通过，plan明确拒绝伪造release CapturedSubject的focused反例通过，随后fmt/clippy通过。未用生产者/消费者同错互相接受替代结构、严格入口与独立规范化证据。

复验：`cargo +1.85 test --locked --offline --manifest-path crates/guard-protocol/Cargo.toml`；`python -B crates/guard-protocol/scripts/generate_schemas.py --check`。

## 剩余任务

完整六产品/五插件/provider/policy/toolchain发布组合、兼容矩阵、发布签名、真实制品采集provider和实际发布继续由K20等任务承担。K07专业报告投影/差分迁移、K09批准验签和可信提供者也未由协议解析实现。release JSON解析仍UNVERIFIED，不证明来源commit/tree关系、实际制品字节或构建可复现。K06 runner/运行账本当前仍局部候选，任务未勾选。K01局部完成不代表CON/EVI整条需求或P0集成可交付。

后续状态：以上为K01验收时点记录；K06之后已取得[独立局部验收](EG-K06-evidence.md)，当前完成计数见[状态页](status.md)，不据此扩大K01范围。
