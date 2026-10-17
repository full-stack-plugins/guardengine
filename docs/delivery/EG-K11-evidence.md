# EG-K11 公共命令行接入证据

事实源：CodeGuard `add-engineering-guard-core` 的 EG-K11、EG-CON-005、EG-GATE-007。任务仍未完成；本页按增量记录证据，不把前置组件通过视为六组 CLI 已交付。

## 固定 Git 执行器

`guard-subject` 增加可选、进程内不可变的 `configure_git_provider`：显式绝对可执行文件、SHA256 和清空继承后的环境。每条 Git 命令前后重新核对制品，所有核对和执行共享原来的采集期限。默认 SDK 未配置时保持原行为；首次使用默认来源后禁止晚切换。环境拒绝 `GIT_*`、大小写别名、非法键及超限输入。配置或摘要本身不产生授信。

真实进程用例验证候选目录和 PATH 中的假 Git 不影响固定来源、显式环境确实清除继承值、默认来源冻结、配置不可变，以及执行前后制品篡改拒绝。Windows junction 的祖先路径拒绝已实测；Unix 链接测试已编写，等待跨系统 CI 实际执行。不声称这些检查提供 DLL 全链校验、OS 原子隔离或消除同用户并发替换。

2026-10-17 作者最终 Rust 1.85 锁定依赖离线串行回归：2 单元、8 provider（含 1 个子进程 worker 入口）、7 materialization、9 real_git，共 26 个 reported，通过且零 ignored。对应耗时为 1.19、49.54、148.48、79.74 秒。格式、all-features/all-targets 严格 Clippy 和 diff 检查通过。根独立执行同一最终单元测试程序：2/2 通过，0.87 秒，包含实际 junction 和截止期限反例。新公共 API 缺失时先有编译失败 RED；固定来源的成功与篡改拒绝均经实际进程验证。

固定 Git 来源提交 `4684537ec950354450f0a20c37cbf19b28d0c25c` 的 [CI 38997382891](https://github.com/full-stack-plugins/guardengine/actions/runs/38997382891) 已终态 success，21 个 job 全部成功，包含两系统 subject 及真实 OPA；该结果只覆盖已发布的组件增量，尚未包含 CLI。未关闭 EG-K11；其余 CLI 接线、真实 OPA 的允许/拒绝、来源验证、退出映射、副作用和文档仍需完成与复验。

## CLI 候选实现与独立审查

新增 `guard-cli`，可执行文件名 `guardcore`。六组命令直接调用现有 schema/契约编译、真实候选和冻结计划、Runtime v2、私有 Store、独立 ProducerVerifier 和 BoundPolicyEvaluator；旧 `guardengine` 及 CodeGuard CLI 保持。缺宿主/批准/工具/政策保持不完整，无替代执行器或自报授信回退。

生产读取系统 KnownFolder ProgramData/GuardCore 或 Unix /etc/guardcore；测试 override 仅非默认 feature 的编译期值。原件与端口闭合、有界、拒绝重复键/链接，操作内冻结整个宿主配置并逐次重读，防止身份与映射混用。独立批准、来源注册与撤销使用 K09 实际核验端口；跨重启不可回退仍是受保护发布端前提。固定 Git 描述与批准环境的标记进入实际候选环境身份，不继承调用方 PATH 来选择 Git。状态/临时根与候选/宿主、彼此均分离。

所有报告仍 `hostProtection=unverified`、`mode=cooperative`、`quality=not_evaluated`、`grant=null`；公开文件端口不证明 OS 强制保护。来源签名不等于质量结论，OPA ALLOW 也不自动签发 Grant。CLI 不提供签名私钥、批准签发、安装或迁移入口。详细命令、字段、退出及副作用见 [CLI README](../../crates/guard-cli/README.md)。

独立只读审查发现并推动实际 RED→GREEN：已有取消文件未同步生效；工具 exit0 但后置复核失败或仅 AuditOnly 仍误退出0；doctor 未分别报告发现维度。最终映射要求 postcheck 完成且 Applied；缺宿主 doctor 也分别报告 CLI/平台实测与其余 not_inspected，不运行 probe。

## 实际本地 CLI 验证

首轮真实签名/Git/SQLite/OPA组合在政策 DENY 误退出0时失败（182.15秒）；修复后完整组合 1/1 通过（283.21秒）。最终扩展组合 2/2 通过（386.66秒），含独立签名批准、私有 Commit 与脏工作树差异、计划恢复与重复 operation 拒绝、真实 OPA ALLOW/DENY/undefined、来源当前性与撤销、伪 Git、批准环境变化、宿主 actor/task 切换、根重叠及输入/数据库分类故障。

该轮还实际等待原生程序 flush 后取消，验证130和部分原始双流保留；错误 operation 取消请求返回2且不派发；真实30秒超时返回3并保留输出。Windows 控制台 Ctrl+C 未实测，不以显式取消文件测试替代此结论。监视任务 RAII 释放，仅覆盖 run/policy evaluate 执行阶段。

完整组合后补充的缺宿主 doctor、Unix 非阻塞/不跟随链接读取、政策 facts JSON 预解码及坏取消输入，经最终编译和相关轻量检查验证；不冒称386.66秒运行已覆盖随后修改。作者最终生产默认6/6（4单元+2真实CLI，后者2.07秒）、feature单元4/4（0.01秒）、all-targets/all-features严格Clippy及fmt通过，均零ignored。根在停写后独立执行最终feature单元程序4/4通过（0.01秒），并验证实际 `guardcore 0.1.0` 可执行及CI结构。Unix FIFO负例仍需Linux实际运行。

新CI增加两系统生产CLI检查，并把真实六组CLI联调接入两系统必需OPA作业，共23个job。测试先写入job私有编译期宿主根及实际Git路径，复用官方固定OPA下载及SHA核验；无缺工具skip。提交后的最终跨系统结果仍待验证，EG-K11保持未勾选；这些本地结果不代替完整路线、宿主或发布验收。
