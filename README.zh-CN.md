# GuardEngine

[English](README.md) | [简体中文](README.zh-CN.md)

GuardEngine 是 Partme Guard 的**确定性规则、契约、证据评估内核**。它不负责解析 Java/Rust 代码，不调用大模型，也不自行决定 Git 合并或宣称架构设计最优。

## 工程内核候选模块

GuardEngine是逻辑GuardCore角色的实现仓库，既有alpha CLI继续保持原行为。以下独立库处于候选实施阶段，其API不建立生产信任或执行获授权的交付动作。

| 包 | 职责 | 说明 |
|---|---|---|
| guard-protocol | 闭合版本化文档、严格原始JSON、JCS和无损路径 | [协议](crates/guard-protocol/README.md) |
| guard-contract | Draft2020-12校验、批准规则组合和验证器准备检查 | [契约](crates/guard-contract/README.md) |
| guard-subject | 实际工作树/暂存区/提交/合入输入及Git对象完整性 | [候选](crates/guard-subject/README.md) |
| guard-store | 私有SQLite/原件存储、幂等及进程中断恢复 | [存储](crates/guard-store/README.md) |
| guard-plan | 批准任务基线、完整冻结义务与显式CAS迁移 | [计划](crates/guard-plan/README.md) |
| guard-runner | 有界原生argv、聚合输出预算与受管进程树清理 | [进程](crates/guard-runner/README.md) |
| guard-run-ledger | 本地持久运行序号、单飞与迟到结果拒绝 | [运行账本](crates/guard-run-ledger/README.md) |
| guard-runtime | 注册计划派发与私有原始输出落盘 | [运行接线](crates/guard-runtime/README.md) |

跨仓[交付清单](docs/delivery/status.md)保留原生需求、场景和任务归属。库存一致性或局部库测试不能关闭全部架构路线；CLI/MCP、可信授权、隔离执行与受控交付仍须分别实现并取得证据。

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

CLI 退出码：`0=ALLOW`、`2=BLOCK`、`3=REQUIRE_APPROVAL`、`4=输入或验证失败`。

## 协议边界

- 契约：`GuardContract`，YAML。
- 事实：`GuardFacts`，JSON。
- 报告：`GuardReport`，JSON。
- V0.1 规则算子：`forbid_relation(subject, predicate, object)`。
- 策略：`enforce`、`review`、`advise`。
- 未知字段/版本/算子一律拒绝，不默认忽略。
- `advise` 只产生提醒及事实，非阻断；`PASS` 只说明未触发阻断，并不表示没有设计风险。

详见 [架构](docs/architecture.md)、[详细技术方案](docs/technical-design.md)、[协议](docs/protocol.md)、[信任边界](docs/security.md) 与 [OpenSpec](openspec/changes/bootstrap-guard-protocol/)。

## 现阶段限制

1. 不含通用表达式引擎、领域模型判断或对象方法调用图。
2. 本地事实可以被伪造，可信 CI 必须基于受保护契约重新分析。
3. 首个 ArchGuard 分析器只对实际读取的 Cargo manifests 生成摘要，不覆盖全部源码。
4. 现阶段没有签名与权威审批机制。
5. 两个独立仓库首次联调采用同级 `path` 依赖，发布后应迁移到固定版本的 GuardEngine 包。

运行 `cargo fmt --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test --all-targets`。
许可证：Apache-2.0。

## Agent installation / 智能体接入

`crates/guard-install` is a separate shared installation library for the six Guard CLIs, independent of the evaluation kernel. See [design and usage](docs/agent-installation.md) and the single [OpenSpec change](openspec/changes/add-guard-agent-installation/).
