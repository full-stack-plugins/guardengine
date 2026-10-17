# 公共内核新增组件的远端CI证据

核对日期：2026-10-17。此页记录具体提交与运行，不把旧提交检查推广到新代码，也不作为全部113任务完成证明。

## 最新核验快照

以下为当前已发布实现；后文旧提交和失败均保留为历史记录。

| 仓库 | 具体提交 | 远端证据 |
|---|---|---|
| GuardEngine | afa10825052e70f4918c6d28bf42a89579f1d313 | [38992692068](https://github.com/full-stack-plugins/guardengine/actions/runs/38992692068)，21/21成功，含双OS真实OPA与批准核验切片 |
| ArchGuard | 42daff5c4edb910a390a5bb7b830ed242cc83b4c | [主CI38988235956](https://github.com/full-stack-plugins/archguard/actions/runs/38988235956)、[安装38988235968](https://github.com/full-stack-plugins/archguard/actions/runs/38988235968)成功 |
| SpecGuard | d4463477b2b665ab7909ff7a67b54acd78810971 | [38972402820](https://github.com/full-stack-plugins/specguard/actions/runs/38972402820)双OS成功 |
| TestGuard | 2100f3e4d5ea079aa91579bdfa5140b99591c90b | [38888948106](https://github.com/full-stack-plugins/testguard/actions/runs/38888948106)成功 |
| GitGuard | 6d017a43c270e768360c6abc2e1f69568c210f86 | [38888942256](https://github.com/full-stack-plugins/gitguard/actions/runs/38888942256)成功 |
| FlowGuard | 608f1c8 | [OPA38990336275](https://github.com/full-stack-plugins/flowguard/actions/runs/38990336275)、[安装38990336276](https://github.com/full-stack-plugins/flowguard/actions/runs/38990336276)双OS成功 |
| CodeGuard | 640d2b9d（最新修复候选） | 前一提交4a82e922的[38990883250](https://github.com/full-stack-plugins/codeguard/actions/runs/38990883250)终态失败：真实CLI/同次Maven、MSRV、默认suite和npm成功，但后续WASM Go测试仍要求历史0.49版本。640d2b9d修正当前输出断言及CVE报告导出版本条件；完整新CI待验收，不宣称全绿 |

OPA历史真实失败及修复见[EG-K08](EG-K08-evidence.md)：首次Flow发布竞态、后续Linux软件SHA后置复核超时与Windows存储准备竞争均保留，不把exit0或本机测试替代完整当前CI。ArchGuard/SpecGuard已完成的最小任务分别见[EG-A01](EG-A01-evidence.md)、[EG-S01](EG-S01-evidence.md)，不扩大为全路线交付。

## 已通过的确切提交

- 仓库：full-stack-plugins/guardengine，main。
- 提交：`0981705d3dfbfa472cbc60d02c16d3ebaf5094ea`。
- [真实GitHub CI运行38963571148](https://github.com/full-stack-plugins/guardengine/actions/runs/38963571148)：completed / success，15/15 jobs成功。
- 七组件guard-contract、guard-subject、guard-store、guard-plan、guard-runner、guard-run-ledger、guard-runtime各在Ubuntu24.04和Windows执行Rust1.85格式、严格Clippy和全feature测试，共14格。
- 另1个root job执行原产品检查、共享installer/evidence、协议测试、Schema生成一致性、ECMAScript JCS对照和库存检查器测试。

该结果补充本地证据，实际覆盖这些组件的Linux与Windows运行分支。它不证明K19要求的完整macOS/多架构制品、恶意进程隔离、签名发布或真实宿主安装。

## 失败与修复历史

`f55501b9d97abe86b2360a2ca88464f9c4631f32`的[首次运行38960965176](https://github.com/full-stack-plugins/guardengine/actions/runs/38960965176)为13/15成功：Windows账本worker在准备阶段busy被unwrap为panic，以及runtime测试以路径字符串前缀比较不同Windows表示，均失败。

`567a36d821d2540e818c98f3eff730efcf3ec418`修复账本准备/竞争同步，后续`0981705`修复目录身份断言。两次修复保留生产Store busy上限、执行预算与路径验证规则；没有删测试或把失败条件改成成功。局部测试及完整最新远端15格成功后才记录通过。

## 专业产品验证过程（历史快照）

CodeGuard `d978e3b50aa3e18ec0d4e2780617562adf2a2d68`的[运行38964163817](https://github.com/full-stack-plugins/codeguard/actions/runs/38964163817)已结束为failure：msrv通过，106份Schema闭包和17项投影测试通过，真实CLI缺工具smoke被精确Schema拒绝，后续真实Maven采集未执行。现生产装配省略未观察的null槽位，但仍声明要求这些槽位存在的历史版本；正在以新精确0.73格式修正版本/形状关联，保留现有presence行为与历史Schema，不恢复假槽位或放开未知属性。尚不能据此关闭K07。

前一提交764e9c41在Linux实际编译发现两个旧Java入口缺observer参数，已各补显式None；该轮真实Maven采集同样未执行，不记录为通过。

后续`a6e91a0080b784d40a4ec3552509b7ba0d86f551`已采用0.73精确版本并通过本地30项投影/装配回归；[实际运行38968207897](https://github.com/full-stack-plugins/codeguard/actions/runs/38968207897)仍终态失败。MSRV通过，但真实缺工具CLI再次被Schema拒绝，Maven步骤仍未执行。正在补充受控fixture原件与静态Schema路径诊断以定位实际差异，不把历史重放成功当作当前CLI成功。

SpecGuard新增三类规格来源观察器后，共享安装测试的精确MCP工具集合增加`specguard_spec_sources`。使用本次实际本地二进制独立复验：10项中7通过、3项因未选择其他产品/provider而跳过，零失败；这不是全部六产品安装矩阵通过。对应SpecGuard新提交的远端CI仍待发布验证。

ArchGuard Java适配器与Rust产品接线仍在本地实施/复验，尚未发布相应新提交的远端验收结果。其他未修改产品的历史绿色检查仍只绑定其原提交，不用公共内核15格替代七产品、五插件或全部交付验收。
