# EG-S01 规格来源只读适配验收

核验日期：2026-10-17。原任务：实现 Spec Kit/OpenSpec/Superpowers 只读来源适配和稳定需求 ID；未知/双事实源冲突不自动选择或初始化。owner 为 specguard；原生唯一任务账本仍为 CodeGuard 总体 change，本页不建立第二份任务勾选表。

| 原任务义务 | 实现与证据 |
|---|---|
| 三来源只读 | 新 `specguard specs sources --project ABS` 与 MCP `specguard_spec_sources` 共享实现；复用原 Markdown/native checker 及有界文件读取，不初始化、安装或写被检查项目 |
| 稳定需求身份 | native key + 作用域 + 框架，分段可逆 UTF-8 percent 编码；换行位置或正文不改ID，不同feature可复用FR-001，重命名不猜等价；OpenSpec基线/增量保留同一逻辑ID及各自来源 |
| unknown/双事实源 | 无来源、自由文本无原生键、读失败或缺覆盖明确 incomplete；多框架 conflict，selected_source 永远 null，不把普通specs目录认成Spec Kit |
| 信任边界 | 固定 local_unverified / not_evaluated / approved_baseline_eligible=false；文档标题、任务勾选、候选位置和退出0均不能提供审批或交付授权 |
| 原行为兼容 | 旧 check/verify 与 trace wire 未改变；原AGENTS/native/trace测试保持，新增工具仅扩充精确MCP集合 |

真实TDD：旧CLI缺入口导致10项失败，补实现后通过；缺MCP工具反例失败后接线；空REQ-后缀曾误识别，真实失败后拒绝。独立审查使用另一target运行完整49项Rust1.85测试全部通过；root对0.3.0候选再次完整49项通过。只读扫描真实ArchGuard仓库保留13个原生需求，旧自由标题规格缺稳定键明确exit3，未伪造完整覆盖。

共享安装检查的原精确工具列表曾遗漏新工具，实际1失败；root新增该准确名称后独立7通过、3个单产品范围以外用例跳过，零失败。portable包实际3skills/9MCP，stdio verified，host_loaded=false；未将包结构证明当成真实宿主加载。

发布实现：SpecGuard main `d4463477b2b665ab7909ff7a67b54acd78810971`，CLI及对应插件0.3.0。[真实CI运行38972402820](https://github.com/full-stack-plugins/specguard/actions/runs/38972402820)已终态success，Ubuntu与Windows两个integration jobs均成功，包含实际构建、全回归、安装及stdio包验证。共享安装期待位于GuardEngine `9053c119d78c6039a7717a907ee3f546c21282ed`，其[运行38971200268](https://github.com/full-stack-plugins/guardengine/actions/runs/38971200268)也已终态success。

依上述原任务字面范围，EG-S01可标verified。S02/S03/S04/S05/S06的需求设计测试追溯、语义完整性、受保护基线与审批、迁移映射及全生命周期联验仍未完成，不通过本项关闭，也不宣称EG-SPEC-001整条路线或全部架构完成。
