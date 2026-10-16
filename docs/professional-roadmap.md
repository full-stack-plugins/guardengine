# 六 Guard 专业能力落地路线

以 CodeGuard 的专业分析器、可复验结果和修复流程为成熟度参照。安装、MCP 注册、提示注入、专业检查和可信交付分别验收；不以命令存在或测试数量推导产品已经成熟。以下为 2026-10-16 本地源码能力。

| 组件 | 已实现的专业范围 | 检查与复验入口 | 下一优先缺口 |
|---|---|---|---|
| ArchGuard | 项目架构/领域模型、需求增量、Maven/Cargo 图规则、Java 语法包/分层/类型归属 | model/change/baseline；check/explain/verify | 完整 Maven 有效模型、编译器类型绑定、语义 Design Diff |
| CodeGuard | 原有多语言质量工具与修复流程 | 原有专业 CLI；本批仅共享接入回归 | 按真实分析器和平台持续验证；本批未重新评估完整专业能力 |
| GitGuard | 复用激活 GitFlow 的分支、合并方向、提交消息/身份规则；安全读取不足不默认允许 | policy check；同源只读 MCP；修正请求后复验 | 固定合入候选、远程保护与规则证据签发；含执行 filter 的仓库需可信隔离 |
| FlowGuard | 十阶段只读预检；显式外部公钥与策略绑定任务、计划、runner、必需测试、当前源码/产物及有效期 | stage status/check；evidence check；技术齐备仍 requires_approval | 真实宿主审批身份、受保护生产者与受控转移 |
| SpecGuard | AGENTS 约定、原生规格结构与摘要；原生需求→源码声明→受信执行用例关联 | agents/specs；trace check/verify | 编译器符号绑定、业务语义验证和受保护基线 |
| TestGuard | 严格 JUnit 导入；显式授权的真实 Maven/Cargo 执行、日志、新报告、源码快照及签名回执 | verify-junit 保持 consistent_untrusted；run/verify、只读 verify MCP | 更广 runner/报告支持、受隔离生产者和覆盖证据；当前本地签名不构成沙箱 |

共享 GuardEngine 保留确定性契约/事实/报告协议，v1alpha1 兼容；v1alpha2 提供五类图规则和覆盖声明。各工具复用 DomainTool 注册真实只读 MCP，不用统一安装成功替代领域检查。

## 推进顺序与完成门禁

1. TestGuard：优先补明确授权、精确 argv/环境/超时/产物范围的真实执行器；从导入核验推进到可追溯执行，先 Maven 后 Cargo。不得把一致但未受信报告提升为执行 PASS。
2. SpecGuard：连接已有需求 ID、ArchGuard 元素 ID 和 TestGuard 用例，区分结构、追踪覆盖和语义实现；保留原生规格唯一事实源。
3. FlowGuard：在已复用阶段规则上消费受信生产者证据，绑定 task/change/candidate，缺审批与缺技术证据分开报告。
4. GitGuard：固定真实提交/合并候选和规则版本，扩展受控 CI 校验；只读本地策略允许不等于远程写权限。
5. ArchGuard：保留静态路径，增量支持有效模型和编译器类型证据；模型生成明确 Maven/JDK、参数、来源与执行权限，不静默运行构建插件。
6. CodeGuard：继续作为成熟实践参照；本批不重写检测器，也不把其共享安装二进制验证扩大解释为全量专业功能复测。

每批交付：规格增量 → 失败测试 → 最小实现 → CLI/MCP 同结果 → 真实项目负例/修复回归 → 插件指引与能力清单。不得用未覆盖图、未经执行的测试、占位扫描器充当 ALLOW。

规格映射：首批专业检查来源为 [professional-guard-checks](../openspec/changes/professional-guard-checks/)，真实执行与证据追踪增量唯一来源为 [execution-trace-evidence](../openspec/changes/execution-trace-evidence/)。ArchGuard 已有增量继续遵循其 project-model-baseline/java-source-architecture；原插件原生规则继续遵循既有规格。本路线记录能力与顺序，不创建第二套业务需求。
