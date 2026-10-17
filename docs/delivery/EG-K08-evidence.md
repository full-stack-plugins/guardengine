# EG-K08 固定 OPA 与 FlowGuard 策略包验收记录

2026-10-17；公共执行器候选 GuardEngine `194df74`，专业政策候选 FlowGuard `4dcadf2`。既有OpenSpec EG-K08为事实源；无新规格体系、无任务预先勾选。

公共内核guard-policy固定OPA1.21.2、原件SHA256、literal argv、闭合包/JCS身份、纯builtin能力与环境/预算。Bound入口要求真实CompiledContract和RegisteredPlan，核对批准pin、当前候选及active head；入口统一绝对截止不在version/eval续期。私有输入/包/双流分别存证；操作意图在派发前CAS持久化，中断重开不静默重跑同operation。无定义、错误、超时、取消、截断、异常JSON和输入漂移撤销可用结论。观察始终UNVERIFIED，不签GateDecision或ExecutionGrant。

FlowGuard独立拥有production Rego和阶段/批准/精确例外规则；内核不定义流程语义。已知违规优先且保留技术缺口；不可豁免项不能由mode或例外放行。当前没有GuardResult到受信facts的生产归一化器，L02与K09保持后续任务，测试批准接口/facts不冒充生产权威。

本机实证：

- 官方Windows/amd64 OPA仅位于用户授权的E盘项目测试目录，官方checksum/固定SHA/大小及真实version已核验，未改全局PATH。
- 作者Rust1.85全套12/12；根独立终态94676再次12/12，其中bound真实Git/SQLite/OPA48.61秒，真实OPA组5/5为211.79秒。
- 28项组合差分中27项实际进入OPA，1项无效信封提前拒绝。FlowGuard独立测试另实际39个Rego和28个直接eval全部通过，不混算成额外Rust测试。
- 准备排队250ms旧实现续期真实RED后修复；缺Map字段panic真实RED后5种负例返回Err；实际timeout/cancel/truncation、禁http、旧包固定、中断及SQLite重开均实际验收。
- 共享CI准备脚本15/15根独立通过（0.307秒）；仅允许显式GitHubActions RUNNER_TEMP，固定Windows/Linux官方SHA和大小，HTTPS闭合重定向、原子不覆盖、现件拒链接及校验后版本探测。

发布接线：内核默认8组件双OS矩阵之外，新real-opa双OS job强制准备真实工具、导出Flow独立case表，再执行全部真实feature；Flow新双OS job执行原生Rego/表及原CLI、阶段和MCP回归。缺包/缺工具直接失败，无skip/fallback。先发布内核，再发布专业包，跨仓首次依赖竞态失败须保留并在发布后重验。

远端实证：FlowGuard策略提交4dcadf2的OPA双OS运行38989888324成功；其旧安装回归Windows夹具按时间戳命名发生碰撞，未记全绿。修复后608f1c8的[OPA双OS38990336275](https://github.com/full-stack-plugins/flowguard/actions/runs/38990336275)及[安装双OS38990336276](https://github.com/full-stack-plugins/flowguard/actions/runs/38990336276)均终态success。修复仅引入原子唯一TempDir，新增同时间戳反例，根独立6/6通过；生产规则与门禁未降低。

GuardEngine194df74的运行38989875403首轮18/19成功，唯一Linux因先取到尚未发布的Flow旧提交缺case脚本而失败；发布后仅重跑失败job，第二轮真实bound测试版本探针未完整，不能再归发布竞态。Windows真实OPA整套已成功，Linux仍须定位复验。

后续诊断0ab8397确认Linux OPA实际exit0且版本/平台正确，真正拒绝来自未优化64MB软件SHA超过原有两秒后置核验。7b220bc仅优化dev/test sha2并修正存储竞争测试的合法busy处理，生产期限、原件比较与安全检查均未放宽。[GuardEngine CI38991992962](https://github.com/full-stack-plugins/guardengine/actions/runs/38991992962)已在完整提交7b220bc678136c2743f3dc846da1c26a4ec2ed45终态success，19/19 jobs全部成功，包括两OS真实OPA、默认8组件矩阵和原根检查。

据原任务字面，EG-K08执行器、专业策略包、真实正负和差分验收已完成，可标verified；不关闭K09/K10/L02。未验证2秒性能目标、生产授权服务、候选OS隔离或受保护平台强制执行。

后续批准核验切片提交afa10825052e70f4918c6d28bf42a89579f1d313的[CI38992692068](https://github.com/full-stack-plugins/guardengine/actions/runs/38992692068)再次21/21成功，新增authority双OS矩阵并保留两OS真实OPA验收。它补充当前提交回归证据，不扩大K08已验收范围。
