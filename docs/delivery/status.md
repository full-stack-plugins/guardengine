# 跨仓交付清单现状

核对日期：2026-10-17。事实源为各仓既有原生checklist，库存格式guard.delivery-inventory/v2。当前新架构路线113项，已验收11项、102项pending；历史长期任务另行保留，113不是全部历史OpenSpec任务的分母。

## 已验收的精确范围

| 任务 | 局部验收 | 证据 |
|---|---|---|
| EG-M01 | 跨仓登记、原生任务和所有权清单 | [清单证据](EG-M01-evidence.md) |
| EG-K01 | 本地协议与闭合Schema冻结 | [协议证据](EG-K01-evidence.md) |
| EG-K02 | 分层契约编译与明确批准端口 | [契约证据](EG-K02-evidence.md) |
| EG-K03 | 冻结计划、绑定、迁移与当前性 | [计划证据](EG-K03-evidence.md) |
| EG-K04 | 四类Git候选和输入身份 | [候选证据](EG-K04-evidence.md) |
| EG-K05 | 本地私有存储、原件与CAS | [存储证据](EG-K05-evidence.md) |
| EG-K06 | 受管执行、双流、运行顺序及修复复验 | [运行证据](EG-K06-evidence.md) |
| EG-K08 | 固定OPA、Flow专业Rego、正负及组合差分 | [OPA证据](EG-K08-evidence.md) |
| EG-K09 | 独立批准、范围复用与Runtime domain来源核验 | [独立核验证据](EG-K09-evidence.md) |
| EG-S01 | 三类规格来源只读适配与稳定原生ID | [规格证据](EG-S01-evidence.md) |
| EG-A01 | ArchGuard独立入口及最小真实ArchUnit闭环 | [架构证据](EG-A01-evidence.md) |

每项只关闭原任务字面范围。K01仍是candidate-unpublished；已推送源码不等于协议正式签名发布。A01仅完成最小精确包禁止边，不覆盖全部模型、领域行为与方法约束。K08不证明生产批准服务、OS隔离、2秒性能目标或全部平台已强制执行。

K07当前投影/同次Maven、默认套件及npm已有实际成功，WASM当前协议断言修正后的完整CI仍待终态，保持未勾选。K09批准与Runtime domain来源核验已于f03336d取得新双OS全绿；只关闭核验端口和提供者范围。K10控制端隔离/授权消费、L02专业归一化与组合、K11新公共CLI以及后续专业/宿主/发布任务均保留。

## 库存范围及权威边界

登记七产品及五关联插件，共12仓。GuardCore逻辑角色复用guardengine，不新建或重命名仓库。CodeGuard原跨仓账本62项；四插件原生集成change及FlowGuard原计划补充共51项；总113稳定ID，唯一原生完成账本和原任务语义不变。

共21份活动原生checklist；12份正式规格登记66条Requirement、89个WHEN/THEN局部场景及43个端到端用例。无稳定EG ID的历史长期任务独立汇总，不造ID、不把原勾选当运行证据。四个集成change为integrate-engineering-guard-facts/host/review/git；Flow沿用已有Superpowers，不初始化OpenSpec。

ownership显式引用CodeGuard所有权表，不在库存中擅自改派。自动发现活动openspec/changes与docs/superpowers/plans，排除archive；正式规格路径显式登记。拒绝场景丢失、未知依赖、重复物理身份、错误owner或来源损坏。

重建前audit.before-reconstruction.json保留62项和31条未解析引用（24个唯一缺失ID）的失败快照，没有删除旧引用或伪造已完成依赖。清单结构通过只证明库存一致，不证明自然语言语义、运行证据真实性、全部任务或生产能力通过。

## 实际复核

在guardengine执行：

```powershell
python -B scripts/verify_delivery_inventory.py --repositories-root E:/workspaces/workspace-agent-skills/full-stack-plugins-repositories --inventory E:/workspaces/workspace-agent-skills/full-stack-plugins-repositories/guardengine/docs/delivery/inventory.json --audit
python -B scripts/verify_delivery_inventory.py --repositories-root E:/workspaces/workspace-agent-skills/full-stack-plugins-repositories --inventory E:/workspaces/workspace-agent-skills/full-stack-plugins-repositories/guardengine/docs/delivery/inventory.json
python -B scripts/tests/test_delivery_inventory.py
```

本次审计与严格模式均实际PASS：113任务、零未解析依赖。库存检查器22项测试中21通过、1项Windows链接能力相关跳过；OpenSpec add-engineering-guard-core严格结构验证通过。以上结果不证明全部任务实现。

审计结果以audit.json本次实际输出为准。实际产品接收、规则owner切换、旧位置指针、全语言资格、真实宿主、受保护CI、四平台签名发布仍按各任务验收，不以库存在、tasks勾选或本地Hook代替。

## 远端原生规格复核

2026-10-17 同步 CodeGraph 插件时发现远端 22ca12d 已含原生集成规划；原先本地检出与可用历史中的缺失不能解释为远端也缺失。已以合并提交 1f328b9 保留远端规格及较新的 v0.2.0 实现，原生文件与远端 8c0902f 完全一致；本地重建版本仅留在 Git 历史中，不替换正式规格。35 项现有测试及该变更 OpenSpec 严格校验通过。清单按原生 F 系列依赖、Requirement 和具名场景重算后严格审计 PASS：113 项任务、66 条 Requirement、89 个场景、43 个端到端用例、零未知依赖。94 是旧本地重建快照计数，不是当前分母；F 系列全部保持 pending。该清单调整不表示专业能力新增验收。

同步CodeGuard插件已发布main时，还发现两份已有原生长期账本：2026-10-03-rust-wasm-runtime-candidate、2026-10-04-rust-lifecycle-default。两者按native_tasks登记，保留已有勾选/缺口，不计入113项新路线的已验收数；活动账本因此由19增至21。
