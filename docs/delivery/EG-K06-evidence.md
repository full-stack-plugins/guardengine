# EG-K06 本地执行与运行顺序验收证据

核验日期：2026-10-17。原任务：实现有界Tokio runner、整棵受管进程树取消、运行序列和部分结果，验证超时、输出截断、迟到PASS与崩溃恢复。验收限于该任务的本地机械机制，不关闭专业投影、受信批准、产品CLI或平台矩阵任务。

| 原任务义务 | 实际实现与可观察证据 |
|---|---|
| 有界runner | guard-runner固定native argv/env、可执行文件前后摘要、共享Semaphore、单调绝对截止、双流聚合输出预算；排队和输入准备消耗同一派发预算 |
| 整棵受管树取消 | Windows暂停创建后先加入Job再恢复；五个真实进程、含分支及孙进程，覆盖取消/超时/正常父退出/输出超限；run返回后第一项检查以已保留句柄确认全部终止，不先sleep |
| 部分结果 | 超时、取消、非零和截断保留已观察原始前缀；runtime将stdout/stderr各自存为私有原字节，不展成JSON数值数组；16KiB及4MiB二进制账本反例通过 |
| 运行序列与迟到PASS | 真实SQLite完整head CAS、服务端分配递增sequence、Started唯一派发、Existing不重放；旧ticket迟到终结为AuditOnly，不复活旧资格；输入回到旧字节也不能恢复旧运行头 |
| 崩溃恢复 | 真实宿主强制退出验证Job关闭清树；runtime future Drop留下Running，显式recover到Incomplete后新operation获新sequence；recover不冒称已证明旧OS树死亡 |
| 实际候选输入绑定 | Worktree/Index/Commit/MergeCandidate逐类物化，真实SHA1/SHA256；合并新blob仅在私有objects存在且确实被执行器读入；原仓index/全部refs/objects不变 |
| 冻结与当前复核 | 工具配置固定logical root/parent模板，实际cwd/argv/env及物化回执另绑定；运行前后复验原source和私有原输入，已确认漂移使观察Incomplete但保留原输出 |

## 真实反例与修复

完整审查历史保留在[EG-K06-review.md](EG-K06-review.md)，没有用新结果改写旧失败。主要实际RED包括：大结果JSON数组超界、Commit身份读取dirty工作树、Windows私有Git index.lock路径失败、Git ceiling等于cwd不能阻断祖先发现。最终v2显式要求 `GIT_CEILING_DIRECTORIES={{subject_parent}}`；不静默把旧root模板渲染成parent。受限Git发现false及解除限制true的同工具配对均真实执行。

原生退出0仍为Incomplete，等待专业适配器；观察信任固定local_unverified，不授质量PASS、批准、ExecutionGrant或Git写权限。

## 测试证据

- guard-runner：Windows Rust1.85共18 reported通过，其中2项是辅助进程入口；实际行为16项。fmt、严格Clippy通过。
- guard-run-ledger：Windows Rust1.85共10 reported通过，其中1项是辅助进程入口。并发、CAS、迟到与恢复均使用真实SQLite/进程，严格Clippy通过。
- guard-subject：最终新增物化7项及原Git9项均通过；root独立四候选SHA1/SHA256物化复验1项通过，105.52秒，退出0。
- guard-runtime：修复前完整候选12项E2E为11通过/1失败，358.81秒；唯一失败为真实Git ceiling对照。修复后受影响4项v2 E2E全部通过，174.02秒，另外8项旧用例filtered。4项单元通过，fmt及严格Clippy通过。
- root独立复验同组4项v2真实E2E：164.50秒，4 passed/0 failed/0 ignored/8 filtered，退出0。随后当前源码4项unit及all-features/all-targets严格Clippy再次通过。未将分阶段证据说成一次全套16项命令通过。

复验使用固定Cargo.lock、Rust1.85、`--locked --offline`，缓存/target/TEMP/TMP位于E盘。具体入口见三个库README和审查记录。当前Windows验证不推断Unix实际通过；CI执行结果及其他平台的取消/隔离属于独立后续证据。

## 保留边界

Job或进程组不等于文件、网络和凭据沙箱；Unix逃离进程组、跨宿主中止、物理断电和恶意同用户ABA未获本次证明。私有目录保留/清理不承诺宿主骤死必执行Drop。存储失败必须传出错误，不声称原件已完整保存。真实身份、独立授权、OPA、专业报告解析、CLI、真实Java试点与发布组合分别留给K07及后续任务。

本次只关闭EG-K06，不关闭整条EVI/GATE需求或总体P0路线。协议保持candidate-unpublished，不执行总体sync/archive。

## 首次远端CI与测试准备修正

提交f55501b的真实CI运行38960965176中，Ubuntu七组件通过；Windows账本双进程测试遇到worker打开数据库的 `store busy`，旧测试unwrap退出101，未到运行票竞争。生产默认300ms busy上限及拒绝派发行为正确，未提高生产等待上限。

测试宿主改为仅对明确的打开阶段busy最多重试5秒；两个worker均完成打开后，以有界ready/go屏障同步真实start竞争。未知错误仍失败，不再把任意start错误都归作正常落败；结束额外核验唯一Running头sequence=1。当前Rust1.85本地账本完整10 reported再次通过，1.07秒，退出0。远端复验仍需按修复提交独立检查，不能把首次失败写成成功。

同次远端Windows runtime还在测试路径断言失败：有效原生cwd与scratch的路径拼法不同，旧 `starts_with` 比较不能证明目录身份。修正测试为核验cwd确属inputs、其真实存在的祖先规范身份等于配置scratch，并主动使用规范化scratch输入覆盖Windows verbatim/DOS差异；生产代码未改。四候选真实执行focused再次通过，158.72秒，1 passed/11 filtered、退出0。首次CI最终13/15成功、两项测试失败均保留原记录，修复提交仍待远端复验。
