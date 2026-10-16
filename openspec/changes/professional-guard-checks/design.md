# 实施职责与信任边界

## 仓库与事实源

| 仓库 | 本批职责 |
|---|---|
| gitguard | 复用 gitflow-plugin 已激活策略，检查 branch/merge/commit；不改工作树 |
| specguard | 明确选择 OpenSpec/Spec Kit，审计原生结构并绑定有界产物清单/摘要，复验旧观察快照 |
| testguard | 按显式计划核验 JUnit/Surefire 报告、候选摘要和必需用例；导入结果不宣称真实执行来源 |
| flowguard + flowguard-plugin | 复用现有十阶段规则，安全只读发现/门禁接入；不创建第二套流程或批准事实 |
| archguard | 0.5.0 Java/Maven 与 Cargo 回归，维持项目基线/需求增量闭环 |
| guardengine | 已有 DomainTool 接口复用、共享安装/MCP 测试登记、成熟度及跨仓库验收记录 |

各智能体只修改所分配仓库，共享源码和集成脚本由主代理管理。构建目录按智能体隔离，不通过切分支、回滚用户修改或修改全局环境来协调。

## 检查与证据

每个专业接口 CLI/MCP 共用实现；context/安装成功仍是 not_evaluated。未配置、漂移、解析不完整、未知策略不默认 ALLOW。文件/用例存在、结构校验、导入报告计数一致、命令退出零各自有明确范围，不推导实际实现/执行/审批结论。

SpecGuard 观察快照用于陈旧内容检测，不能充当批准基线。TestGuard 导入报告返回 execution_verified=false，即使结构一致也仅 consistent_untrusted。FlowGuard 复用现有规则时保持 allowed/denied/approval 缺口的含义，不能以伪造 actor 自批。

GitGuard 和 FlowGuard 不执行 Git 写操作、业务构建或发布。插件适配器不得从未受信项目文件选择任意可执行路径，不能将超时、畸形输出或适配器缺失降级为成功。

## 验收方法

专业负例→修复→重新检查/复验；真实 stdio MCP 与 CLI 一致；路径、资源预算和输入漂移负例；六工具受管上下文共存、安装更新/卸载及旧流程兼容。最终分别记录实际通过的测试与未完成能力，不以安装能力替代专业门禁。
