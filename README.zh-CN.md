# GuardEngine

[English](README.md) | [简体中文](README.zh-CN.md)

GuardEngine 是六个独立专业守卫共用的**确定性规则、契约、证据评估内核**。它不负责解析 Java/Rust 代码，不调用大模型，也不自行决定 Git 合并或宣称架构设计最优。

## 本地集成实现（待审阅分支）

本分支已增加独立 `guard.integration/v1alpha1` 信封、绑定尝试生命周期、有界证据重算与生产者评估、借用字段 `FactBudget`、纯资格/尝试存储端口，以及 Unix 私有目录的单次工件发布。原生命令和旧协议保持不变。身份端口不代表真实身份提供方，内存 CAS 不代表持久存储；六守卫的冻结本地协议矩阵已核验；生产能力、宿主强制检查与公开发行仍未完成。

参见[实施证据](docs/implementation-progress.md)、[集成 YAML 限制](docs/yaml-integration-profile.md)、[事实构建预算](docs/fact-construction-budget.md)、[信任 API](docs/integration-trust-api.md)、[独立本地制品验证](docs/local-artifact-release-adr.md)。下文原始 main 核验记录保留历史事实，集成能力的当前状态以本节和实施证据为准。

## V0.1 / Guard Protocol v1alpha1

- **Contract Engine**：加载和校验带版本的 YAML 工程契约。
- **Rule Engine**：对专业 Guard 生成的事实执行关系规则。
- **Evidence Engine**：输出包含规则摘要、事实摘要和稳定评估 ID 的 JSON 报告。
- **Analyzer Interface**：定义 `GuardAnalyzer`，供各 Guard 实现。
- **不确定不放行**：`partial` 事实使规则 `INDETERMINATE`，最终结果 `BLOCK`。
- **不伪造信任**：本地报告固定为 `signed: false`；verify 只证明可重算一致，不能证明可信来源。

## 使用

将 ArchGuard 与 GuardEngine 放在同级目录：

```text
workspace-full-stack-plugins/
├── guardengine/
└── archguard/
```

在 `archguard` 目录执行：

```sh
cargo run -- check --project fixtures/forbidden --contract examples/agent-job-contract.yaml --facts facts.json --report evidence.json
# 预期退出码 2，阻断；仍然生成报告。

cd ../guardengine
cargo run -- verify --contract ../archguard/examples/agent-job-contract.yaml --facts ../archguard/facts.json --report ../archguard/evidence.json
# 能重新验证内容一致，但决策仍是 BLOCK。
```

CLI 退出码：`0=ALLOW`、`2=BLOCK`、`3=REQUIRE_APPROVAL`、`4=输入、运行或验证失败`。

## 协议边界

- 契约：`GuardContract`，YAML。
- 事实：`GuardFacts`，JSON。
- 报告：`GuardReport`，JSON。
- V0.1 规则算子：`forbid_relation(subject, predicate, object)`。
- 策略：`enforce`、`review`、`advise`。
- 未知字段/版本/算子一律拒绝，不默认忽略。
- `advise` 只产生提醒及事实，非阻断；`PASS` 只说明未触发阻断，并不表示没有设计风险。

详见 [架构](docs/architecture.md)、[技术方案](docs/technical-design.md)、[共享集成契约草案](docs/integration-contract.md)、[协议](docs/protocol.md)、[信任边界](docs/security.md) 与 [OpenSpec](openspec/changes/bootstrap-guard-protocol/)。

## 现阶段限制

1. 不含通用表达式引擎、领域模型判断或对象方法调用图。
2. 本地事实可以被伪造，可信 CI 必须基于受保护契约重新分析。
3. 首个 ArchGuard 分析器只对实际读取的 Cargo manifests 生成摘要，不覆盖全部源码。
4. 现阶段没有签名与权威审批机制。
5. 两个独立仓库首次联调采用同级 `path` 依赖，发布后应迁移到固定版本的 GuardEngine 包。

## 当前实现与目标设计

2026-10-09 核验 main `0284f1ef4bb93e6602d5a65a5341f10e01a63ddf`：已有 v0.1.0 Rust crate 与 CLI，edition 2024、最低 Rust 1.85。依据为 `src/protocol.rs`、`src/engine.rs`、`src/analyzer.rs`、`src/main.rs`；`tests/` 中有 10 个测试函数，本轮未执行。

上述历史 main 核验时，ArchGuard 通过同级 path 使用引擎，另外四个守卫只有设计文档。当前本地分支已有六个独立生产者/适配器，CodeGuard 保留成熟实现。精确版本与能力边界见[冻结本地矩阵](docs/local-consumer-matrix.md)。

可选集成库已实现版本化信封、精确绑定与通过注入认证记录端口执行的通用资格检查，涵盖过期/撤销和追加/CAS 历史。真实身份服务和生产控制器仍未提供。`load_envelope_json` 接受独立 `guard.integration/v1alpha1`；原生 contract/facts/report 加载器不接受此封装。保留历史 `guard.partme.ai/v1alpha1` 是兼容要求，不表示项目名称带旧前缀。

## Library 与 CI 集成

库调用使用 `load_contract_yaml`、`load_facts_json`、`evaluate`、`verify_report` 及类型化错误。专业分析器实现 `GuardAnalyzer`；领域解析与策略含义不进入引擎。

`evaluate` 不传 `--report` 才向 stdout 输出 JSON，传入时写入该文件。`verify` 向 stderr 输出一致性消息，不输出 JSON 成功封装，退出码仍反映原报告决策；一致的 BLOCK 报告退出 2。当前错误为 stderr 文本。每次运行使用新输出路径并检查退出状态，不能因旧报告文件存在便判断成功。

可信 CI 应从受保护来源加载契约，独立分析精确候选并核实覆盖范围。引擎目前不自动执行分支保护、认证审批或使过期结果失效。CodeGuard 现有退出语义不同，需要按命令适配。

## 验证状态与贡献

本轮检查源码、文档链接和命令声明。较早架构审阅阶段的云端 PATH 中没有 Cargo/OpenSpec，该阶段未运行功能测试或 OpenSpec 验证，不宣称通过。后续规划阶段的验证另见下方记录。技术方案包含事实清单、边界和分阶段可测验收。

有对应工具链时运行 `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets`。
许可证：Apache-2.0。


## OpenSpec 实施待办

新增增量 [proposal](openspec/changes/add-versioned-guard-integration-contracts/proposal.md)、[design](openspec/changes/add-versioned-guard-integration-contracts/design.md)、[规范](openspec/changes/add-versioned-guard-integration-contracts/specs/) 与 [tasks](openspec/changes/add-versioned-guard-integration-contracts/tasks.md)，将架构方案拆成待实施工作。参阅[跨仓依赖路线图](openspec/guard-roadmap.md)与[结构验证记录](openspec/validation-2026-10-09.md)。任务清单已记录独立复核的本地实现，本检查点为19/24，其他任务保持未勾选。当前62项测试及精确边界见[实施进展](docs/implementation-progress.md)。前文源码树清单和验证限制对应检查基线或较早的架构审阅阶段；本次另行新增 OpenSpec 文档并记录实际 CLI 校验。既有 change 的任务归属和历史完成证据继续保留。
