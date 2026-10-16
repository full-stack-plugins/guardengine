# EG-K06 执行闭环验收审查

状态：原任务本地机械范围已验收，最终结论见[验收证据](EG-K06-evidence.md)。以下保留审查时的分阶段观察与失败，不将历史未完成状态覆盖成通过。沿用CodeGuard临时跨仓账本的EG-K06，不新增任务事实源。

原任务要求：有界 Tokio runner、整棵受管进程树取消、运行序列、部分结果，以及超时、输出截断、迟到 PASS 和崩溃恢复反例。

## 逐项核验

| 义务 | 当前实现与证据 | 尚需确认 |
|---|---|---|
| 有界进程执行 | guard-runner 固定参数及环境、工具摘要、共享并发、单调绝对截止和聚合输出上限；Windows 本机18 reported测试通过，其中2个为辅助入口 | Unix实际运行归独立平台验证，不以Windows结果推断 |
| 整棵受管树终止 | Windows暂停创建、先加入Job再恢复；真实五进程树覆盖正常父退出、超时、取消、输出超限，返回后第一项检查为全部句柄已终止；宿主强制退出也有真实测试 | OS文件/网络/凭据沙箱和恶意进程逃逸不是本库保证 |
| 部分结果 | 原始stdout/stderr分别以私有字节保存；超时、取消、非零和截断保留已观察前缀；超过2MiB输出不转成JSON数值数组 | runtime最终v2全套受影响回归待终态证据 |
| 持久运行序号与迟到结果 | guard-run-ledger 真实SQLite/CAS；Started才派发，Existing不重放，旧ticket终结仅AuditOnly；10 reported测试通过，其中1个为辅助入口 | 不将本地Pass状态当受信交付授权 |
| 崩溃恢复 | runner宿主强制退出清Job；运行future丢弃留下Running，显式recover将旧运行置Incomplete，再由新operation获得递增序号 | recover本身不证明OS树死亡；控制端须先确认终止或隔离 |
| Git候选与执行字节一致 | guard-subject物化Worktree/Index/Commit/MergeCandidate，包含SHA1/SHA256及仅存在私有objects的合并新blob；新增7项及既有9项实际通过；root独立四候选物化复验通过 | runtime真实读入四候选待最终结果 |
| 冻结配置与实际调用关联 | v2将逻辑subject_root模板固定在批准配置中，实际私有cwd/argv/env另存摘要与物化回执；Git发现ceiling显式固定 | 最终v2回归、独立源码审查及任务字面复核完成后才能验收 |

## 已发现并修复的问题

1. 运行账本把原始结果展开为JSON数组，较大二进制结果触发数组长度限制。真实反例失败后改为以原字节摘要构造键，原件仍保存原字节，16KiB及4MiB反例通过。
2. 冻结Commit身份却在dirty工作树运行。真实反例失败后先阻断非Worktree的v1调用，再引入显式v2物化档位。最终解决方案必须验证四类真实字节，不能以永久仅支持Worktree替代原范围。
3. Windows规范路径的verbatim前缀使私有Git index.lock创建失败。实际失败后仅转换Git私有路径环境的等价表示，不修改原始GitPath或候选身份；物化全回归通过。
4. 派发前检查耗尽运行预算时，必须记录TimedOut且无PID，不能伪称输入漂移或补造输出。运行前后UTC时间来自执行器真实观察，未派发保持为空。
5. v2真实Git发现测试复现Windows verbatim路径作为 `GIT_CEILING_DIRECTORIES` 未阻止祖先仓库发现；进一步真实probe证明即使转换路径，ceiling等于cwd本身仍不起预期作用。[Git官方说明](https://git-scm.com/docs/git#Documentation/git.txt-codeGITCEILINGDIRECTORIEScode)也明确不排除当前工作目录。修正为显式冻结 `GIT_CEILING_DIRECTORIES={{subject_parent}}`，只渲染 `TempDir/inputs` 的直接私有TempDir父目录；旧root-ceiling配置明确拒绝，不偷换模板身份。Windows原生路径采用等价DOS/UNC表示；一次性替换不再次解释生成路径中的花括号。真实受限false/解除限制true的配对用例已通过，root独立复验仍在运行。

## 复验命令与边界

```powershell
cargo +1.85 test --locked --offline --manifest-path crates/guard-runner/Cargo.toml --features test-fixture
cargo +1.85 test --locked --offline --manifest-path crates/guard-run-ledger/Cargo.toml
cargo +1.85 test --locked --offline --manifest-path crates/guard-subject/Cargo.toml
cargo +1.85 test --locked --offline --manifest-path crates/guard-runtime/Cargo.toml --features test-fixture
```

构建缓存、目标和TEMP/TMP均位于E盘。上述命令列明复验入口，不代表尚在运行的命令已成功。各包README记录此前真实结果；最终结果有实质变化后更新本文，不回写历史证据。

root独立复验：Rust1.85、独立 `root-subject-review` 构建目录，运行 `guard-subject --test materialization all_four_candidates_materialize_their_real_bytes_without_writing_original_git -- --exact --nocapture`；真实SHA1/SHA256四候选测试于105.52秒结束，1 passed、0 failed、0 ignored，退出0。这证明该项物化行为，不代替runtime实际派发或其他6项物化反例的独立重跑。

root另行运行八个新增组件的Rust1.85格式检查，均退出0；库存严格检查PASS，库存测试22项中21通过、1项因Windows符号链接权限跳过。这些检查证明格式和库存结构，不替代执行器行为或全部任务验收。

runtime v2首次完整候选回归：2项unit通过；12项真实E2E在358.81秒结束，11通过、1失败。已通过真实四候选读取、仅私有objects的合并新blob、nonce变化不改批准配置、来源漂移和私有输入修改；唯一失败是上述Windows Git ceiling正反对照。修复后须重跑受影响的全部v2用例与路径转换单元；不得把这一轮记录为全套通过，也不得据11项通过勾选K06。

修复后的受影响范围已实际复验：`cargo +1.85 test --locked --offline --features test-fixture v2_ -- --nocapture`，4项E2E全部通过、8项旧用例filtered，174.02秒、退出0；另4项unit全部通过、fmt及严格Clippy退出0。分阶段证据覆盖已知修复范围，不改写初轮11/12记录。root随后独立复验同组4项v2行为全部通过，164.50秒、退出0；当前源码4项unit及严格Clippy再次通过后，才更新原生任务及库存为7/113。

本次不关闭K07专业投影、K08 OPA、K09/K10可信授权与动作隔离、K11产品CLI、K12真实Java试点、K19平台矩阵或K20发布组合。当前公共协议仍为candidate-unpublished；未执行总体sync/archive，未将旧提交的绿色CI归给本地新代码。
