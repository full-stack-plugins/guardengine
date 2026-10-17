# 跨仓交付清单现状

核对日期：2026-10-17。当前清单为 `guard.delivery-inventory/v2`，登记原生任务、需求、局部场景及端到端用例。EG-M01的清单建设已按原任务验收，详见[逐项证据](EG-M01-evidence.md)；实际产品接收和权威切换仍未全部完成，不能据清单建设完成推断运行能力通过。

## 权威来源与边界

- 登记七产品仓和五关联插件仓，GuardCore逻辑角色映射已有 `guardengine` 仓库，不新建或重命名仓库。
- CodeGuard临时跨仓账本保留62项总体任务；另按既有规划重建四插件OpenSpec集成change与FlowGuard原计划第5节，共51项。113项任务保留稳定ID和唯一原生完成账本，EG-M01清单建设、EG-K01至K06、EG-S01及EG-A01局部任务已验收，共9/113项verified，其余104项仍pending。K07及其他后续集成任务保持未勾选。
- `ownership: {repository, path}` 显式引用CodeGuard权威所有权表，逐项核对真实owner，不能在清单内擅自改派。路径拒绝空段、尾斜杠、链接和重复物理身份。
- 共19份活动原生checklist；无稳定EG ID的历史长期任务独立汇总，不另造ID或改变其原生状态。
- 12份正式规格来源登记66条Requirement、94个具名WHEN/THEN局部场景，另登记43个端到端用例。校验原生定义、任务引用、场景名及用例责任任务，拒绝丢失场景、未知需求和错误归属。代码围栏中的示例不计为正式定义。
- 自动发现活动 `openspec/changes/**/tasks.md` 和 `docs/superpowers/plans/**/*.md`，排除archive。并非任意文档都自动成为规格来源；每个正式规格路径显式登记。

## 当前审计与重建来源

`audit.json` 是实际执行结果。113项依赖均可解析且无环，库存一致性PASS；这不证明自然语言语义、运行证据真实性、生产能力或任何任务已经验收。

`audit.before-reconstruction.json` 保留重建前的失败快照：62项任务、31条未解析引用、24个唯一缺失ID。原引用未删除，也没有伪造外部已完成节点。

四个新重建change为 `integrate-engineering-guard-facts`、`integrate-engineering-guard-host`、`integrate-engineering-guard-review` 和 `integrate-engineering-guard-git`；FlowGuard沿用既有Superpowers规格/计划追加章节，不初始化OpenSpec。来源是既有traceability/acceptance/protocol及各插件维护边界，不声称恢复了历史原文。

provider兼容验证与最终迁移/发布验收分别保留，避免循环依赖而不减少最终义务。来源链接和重建说明保留在各原生变更中。四个change已通过OpenSpec严格结构校验；文件存在或结构有效不证明实现通过。

## 复核

在guardengine仓库执行：

```powershell
python -B scripts/verify_delivery_inventory.py --repositories-root E:/workspaces/workspace-agent-skills/full-stack-plugins-repositories --inventory E:/workspaces/workspace-agent-skills/full-stack-plugins-repositories/guardengine/docs/delivery/inventory.json --audit
python -B scripts/verify_delivery_inventory.py --repositories-root E:/workspaces/workspace-agent-skills/full-stack-plugins-repositories --inventory E:/workspaces/workspace-agent-skills/full-stack-plugins-repositories/guardengine/docs/delivery/inventory.json
python -B scripts/tests/test_delivery_inventory.py
```

当前审计和严格模式均退出0；任一未知依赖、场景丢失、错owner、重复定义或来源损坏仍非零。检查器不修改清单或原生任务状态。正式实施时仍须运行独立产品、工具、宿主、平台和授权验收。

## 尚未完成

原生定义和Requirement/Scenario/端到端用例清单已补齐，EG-M01只关闭“建立移交清单”的任务范围。实际产品接收、规则owner切换和旧位置指针切换仍按对应迁移任务进行；未接收前保留原生记录，不将其扩大为M01的前置，也不据此关闭产品实现或全路线交付。继续按原依赖推进协议、契约批准、候选与证据、隔离执行、授权、专业域及生产交付。

K06的原任务逐项核验和真实反例见[执行闭环验收审查](EG-K06-review.md)，最终局部验收见[执行与运行顺序证据](EG-K06-evidence.md)。四类候选实际执行与Git目录边界已取得root独立复验，才更新原生勾选；其余专业集成及平台验收继续保留。

## 局部实现验收

2026-10-17独立审查及真实测试完成后，EG-K02、EG-K03、EG-K05按各自原任务字面标为verified，证据分别见[契约编译](EG-K02-evidence.md)、[冻结计划](EG-K03-evidence.md)、[本地存储](EG-K05-evidence.md)。加上[本地协议冻结](EG-K01-evidence.md)、[Git候选采集](EG-K04-evidence.md)、[执行与运行顺序](EG-K06-evidence.md)、既有EG-M01及EG-S01，当前共9/113，104项pending。K01保持candidate-unpublished；全部未来集成任务保持未勾选，不把库完成当作P0集成可交付。

本次库存回归实际22项：21通过、1项Windows符号链接权限跳过；严格模式与audit均PASS、退出0，audit.json已刷新。

2026-10-17新增局部验收：[EG-S01三类规格来源适配](EG-S01-evidence.md)完成真实RED→GREEN、独立全49项回归及SpecGuard具体提交的Ubuntu/Windows双CI验收；现共8/113 verified、105 pending。上述七项内核/清单之外仅新增S01，不关闭S02–S06、K07或A01。

2026-10-17新增ArchGuard验收：[EG-A01独立字节码闭环](EG-A01-evidence.md)已完成本地独立复验与具体提交的Ubuntu/Windows真实Java和Rust E2E及安装CI，现9/113 verified、104 pending。仅完成本任务最小精确禁止边，后续领域/方法、平台和发布义务不变。
