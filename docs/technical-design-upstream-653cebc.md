# GuardEngine — 技术方案与协议演进规划

> 版本 V0.1 / Guard Protocol `guard.partme.ai/v1alpha1`。当前具有可运行的 Rust 实现及单元/CLI 验证；OPA、可信签名、审批、MCP 和集中服务器属于**规划能力**。

## 1. 架构目标

**GuardEngine** 为六个独立守卫提供三个确定性基础引擎：**Contract Engine、Rule Engine、Evidence Engine**。它不直接理解 Java/Rust/TypeScript AST、需求文档、测试覆盖率或 Git 分支；这些分别属于专业 Guard 的领域分析器。各 Guard 默认内嵌兼容版本的 GuardEngine SDK，可以单独安装，不要求常驻服务。

~~~text
Professional Analyzer ── GuardFacts ──┐
                                     ├─► GuardEngine
Approved GuardContract ──────────────┘   Contract / Rule / Evidence
                                             │
                                        GuardReport
                                             │
                                Local CLI / MCP / trusted CI
                                             │
                                  FlowGuard/GitGuard gates
~~~

## 2. 当前代码与职责

[Cargo package](../Cargo.toml)、[protocol.rs](../src/protocol.rs)、[engine.rs](../src/engine.rs)、[analyzer.rs](../src/analyzer.rs)、[main.rs](../src/main.rs) 是当前实现事实源：

- **Contract Engine**：对 v1alpha1 YAML 契约执行严格反序列化、协议类型、必填字段、唯一规则 ID 和枚举校验；不支持的字段/算子拒绝。
- **Rule Engine**：对通用关系事实 `(subject,predicate,object,source)` 执行精确禁止边 `forbid_relation`。根据 `enforce/review/advise` 产生 PASS/FAIL/REVIEW_REQUIRED；partial 事实集合导致 INDETERMINATE。
- **Evidence Engine**：稳定排序/去重事实，以 SHA-256 对合同与事实计算摘要，生成稳定 evaluationId 和 JSON 报告；`verify` 对整份报告**重新计算**而非只比较报告内摘要。本地结果 `signed=false`，无法证明权威 CI 已执行。
- **Analyzer 契约**：Rust `GuardAnalyzer` trait 接受项目根与 subject_id，专业 Guard 返回 GuardFacts。提取源码/编译器语义不属于引擎。
- **CLI**：`evaluate` 和 `verify`，退出码 `0=ALLOW`、`2=BLOCK`、`3=REQUIRE_APPROVAL`、`4=错误`。

这套模型是最小可执行切片；还没有可信的批准快照仓库、长期持久化、进程隔离、OPA 策略后端、数字签名或 ExecutionGrant。

## 3. 协议对象与扩展方式

当前 v1alpha1：
- GuardContract：apiVersion/kind/metadata/revision/rules；
- GuardRule：id/description/enforcement/assertion；
- GuardFacts：analyzer identity/subject/snapshotDigest/completeness/facts/diagnostics；
- GuardReport：engineVersion、contract/facts digests、evaluations、decision、signed=false；
- GuardAnalyzer：由专业工具实现，报告真实观察范围。

详见 [Guard Protocol](protocol.md) 和 [JSON Schema](../schemas/v1alpha1/)。

**后续设计原则：**
1. 协议版本与域规则版本分离。引擎只能解析已声明支持的 Schema。
2. 事实、执行状态、判断结果、覆盖状态分维度表达，不能只保留布尔 `passed`。
3. 每条规则有 owner、scope、required analyzer capabilities、ENFORCE/REVIEW/ADVISE、例外条件。
4. 专业 Guard 可以实现本域集合/图计算，计算完成后输出有来源的事实；GuardEngine 不把 DSL 任意代码执行混进核心。
5. GuardResult（检查证据）、GateDecision（对动作的策略结论）和 ExecutionGrant（受信执行授权）在扩展版本中**分开建模**。
6. 不支持的强制能力必须明确阻断，不能从没有找到匹配边推断合规。

## 4. 执行 Runtime（下一阶段）

待实现的 `ValidationPlan` 应从批准快照固定需要运行的规则、分析器、被检对象与版本要求，**先定义义务再执行检查**。Runtime 负责受控启动、并发预算、超时、取消、完整日志摘要、失败恢复、报告新鲜度；建议按需使用 Tokio/Clap。外部专业验证器接口使用版本化 JSON/进程协议，并可声明工具、输入、资源及网络权限。

当候选源码、策略、检查工具或测试集发生变化，受影响证据必须失效。多租户/多任务共享缓存不得跨身份/作用域复用授权。

## 5. 策略与可信权限

OPA/Rego 属于**可选策略执行适配器**，不等于身份认证器或 Git 写执行器。受信控制端拥有规则批准、例外/撤销、执行者权限与密钥；编码 Agent/工作树只可以提交候选和消费反馈，不能管理放行规则或签发授权。

本地报告一致性与受保护 CI 的信任差异：
- `guardengine verify`：确认当前合同/事实能重算得出相同 JSON。
- 可信 CI：从受信策略基线重新运行真实分析器，在受保护的 candidate SHA/tree 上保存来源、工具身份、原始报告及必要 attestation。
- GitGuard/平台合并执行器：在操作前校验 GateDecision、ExecutionGrant、目标版本与权限。不能让具有完整写权限的 Agent 自己宣布 “enforced”。

未来可考虑 in-toto/SLSA 签名证明、内容寻址存储、签发策略和重放检测；签名只证明产物来源与完整性，不证明规则本身正确。

## 6. SDK/CLI/MCP/CI 形态

保持 **Rust SDK（默认内嵌）+ 独立 CLI（现有）+ MCP/CI（后续）+ 可选服务**。所有入口必须使用同一语义版本的协议和检查器，不因调用介质差异而把 UNKNOWN 降成 PASS。

现有使用：

~~~sh
guardengine evaluate --contract contract.yaml --facts facts.json --report report.json
guardengine verify --contract contract.yaml --facts facts.json --report report.json
cargo test --all-targets
~~~

拟议未来：

~~~sh
guardengine doctor --project .
guardengine contract validate --path approved.yaml
guardengine plan --candidate <tree> --approved-ref <ref>
guardengine run --plan plan.json
guardengine evidence inspect --report report.json
guardengine policy evaluate --subject <candidate> --action merge
~~~

这些未来命令尚不可执行；MCP 不应暴露任意 shell 或默认可写 Git 操作。CLI 能离线检查，集中服务是可选安装，不成为专业守卫独立运行的前提。

## 7. 安全、性能、质量要求

- **输入安全**：拒绝未知 schema、未知版本、非法 Unicode/非预期字段、过大输入；YAML 不得携带可执行表达式。
- **可复现**：合同和事实规范化后序列化；包含规则版本、分析器版本、源输入和环境，检测不同机器规范化差异。
- **运行控制**：有界内存/输出、超时、取消、支持进程树回收；工具异常报告 partial，不强行拼出 PASS。
- **可信来源**：签名/身份/批准由独立控制端；本地 `signed=false` 不等于可合入。
- **依赖关系**：GuardEngine 不编译依赖 Spec/Arch/Code/Test/Git/Flow Guard 的源码；专业适配器可独立版本发布。
- **兼容性**：Guard Protocol/N/N-1、Rust SDK/CLI 语义、跨平台锁定协议执行结果必须验证。

## 8. Wave 计划与验收

| Wave | 状态与交付 | 必须证明 |
|---|---|---|
| E0 | **现有**：v1alpha1 合同、精确关系规则、JSON 报告/CLI 重算 | 合法/违规/review/partial/篡改检查 |
| E1 | Analyzer Capability + Frozen ValidationPlan | 未覆盖或不支持的强制规则不能“空匹配通过” |
| E2 | 多语言专业 Adapter 协议、Runner 超时取消 | 工具失败、部分结果、旧报告和并发隔离 |
| E3 | 受信批准快照、OPA 策略与例外/撤销 | 自改规则、伪造审批、过期例外均拒绝 |
| E4 | 本地内容寻址证据存储、MCP、CI、可信证明 | 签名身份/源 commit 绑定、报告无法重放 |
| E5 | SDK 发布、N/N-1 兼容、独立服务与多平台 | 六守卫独立安装、升级回滚、服务端强制验收 |

先以 ArchGuard 的真实禁止依赖检查做协议回归，再加入 SpecGuard/TestGuard 的集合规则、GitGuard 的精确候选身份和 FlowGuard 阶段审批。规则覆盖范围不同的专业守卫**不必同步实现**所有能力，但其必需规则若没有 Provider 就不得获得 ALLOW。

## 9. 既有实现与规范位置

现有代码：[engine.rs](../src/engine.rs)、[protocol.rs](../src/protocol.rs)；协议：[protocol.md](protocol.md)；安全边界：[security.md](security.md)；OpenSpec 首个变更：[bootstrap-guard-protocol](../openspec/changes/bootstrap-guard-protocol/)。本设计仅补全下一阶段技术蓝图，不改变已实现代码或将未来能力标记完成。
