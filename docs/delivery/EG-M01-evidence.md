# EG-M01 清单建设验收证据

日期：2026-10-17。此记录只关闭“建立规格/任务/场景移交清单”任务，不代表七产品实现、规则迁移、生产资格或整条路线完成。

## 原任务与证明范围

原任务：建立七项目规格/任务/场景移交清单，核实目标仓库和初始化授权；保留稳定ID、旧位置指针与唯一账本，未接收前不删原记录；用重复owner和丢失用例反例校验。

| 义务 | 实际证据 | 结果 |
|---|---|---|
| 核实七产品及关联插件仓库 | repository-discovery.json记录12个实际仓库的HEAD、分支、SDD标识及原生账本，独立审查重新读取Git和文件系统对照 | 符合 |
| 核实初始化边界 | 沿用现有OpenSpec/Superpowers；无体系的产品保留CodeGuard临时账本。未安装/初始化/迁移新SDD，不把CLI安装视作项目初始化 | 符合 |
| 完整任务与原生位置 | inventory.json登记113项稳定任务：原62项总体任务与51项新重建插件定义，19份活动账本均引用原生路径 | 符合 |
| 需求、场景及验收清单 | 12份正式规格、66条Requirement、94个具名WHEN/THEN场景、43个端到端用例逐项核对定义和任务引用 | 符合 |
| 唯一owner与历史ID | 权威ownership表与任务/清单交叉核对；GuardCore映射guardengine，原ID和状态不按名称推断替换 | 符合 |
| 接收前保留旧记录 | 原62项定义及依赖保留原位置；插件旧引用保留并标注新重建来源；无删除或假完成节点。实际接收/权威切换仍由后续任务验收 | 符合 |
| 重复owner/丢失场景反例 | 22项单测中21通过、1因Windows创建symlink权限跳过，覆盖错误owner、物理路径重复、删除具名场景及删除端到端用例 | 符合；跳过项不作已运行证明 |
| 依赖可解析且无环 | 实际严格审计退出0，独立拓扑排序覆盖113项任务，无未知引用或环 | 符合 |

## 实际验证

在guardengine执行：

```text
python -B -m unittest discover -s scripts/tests -p test_delivery_inventory.py
python -B scripts/verify_delivery_inventory.py --repositories-root E:/workspaces/workspace-agent-skills/full-stack-plugins-repositories --inventory E:/workspaces/workspace-agent-skills/full-stack-plugins-repositories/guardengine/docs/delivery/inventory.json --audit
```

四个重建change分别通过OpenSpec严格校验：integrate-engineering-guard-facts、integrate-engineering-guard-host、integrate-engineering-guard-review、integrate-engineering-guard-git。FlowGuard沿用原生Superpowers追加章节，未初始化OpenSpec。独立审查确认原36条AT未改、旧Requirement→Task关联边保留，新增定义未吞掉最终宿主/迁移/发布义务。

行为曾实际失败后修复：同物理路径不同写法可重复owner、无inline owner任务可错派、围栏示例被误当正式任务；回归均通过。旧审计FAIL保留在audit.before-reconstruction.json，不用新校验追认历史原文。

## 未包含的完成声明

本任务只建立移交清单。实际产品接收、规则owner切换、独立运行/真实宿主/受保护CI、来源批准、隔离执行及生产授权须分别证明。库存PASS不是工程放行，EG-PROD-001/003的全部运行场景也没有因此完成。
