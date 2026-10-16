# guard-subject

EG-K04 候选输入采集库，使用 `guard-protocol` v1 类型。实现实际 Git 对象与输入清单定位，不产生 GuardResult PASS、可信 repository 身份、签名或操作授权。尚未发布；不修改 GuardEngine alpha 命令。

## API 与身份

```rust,ignore
let captured = guard_subject::capture(repository_root, &binding, &selection)?;
guard_subject::verify_current(repository_root, &binding, &selection, &captured)?;
```

`repository_root` 必须是绝对、无符号链接／reparse 且精确等于 Git 顶层的本机目录。`CaptureBinding` 必须显式提供 repositoryId、taskId、worktreeId、contractDigest、policyDigest、verifierDigest、environmentDigest。它们是调用端声明，必须由外部受信注册表和控制端验证；本库不会从目录摘要、remote URL 或 `.git` 路径猜测仓库身份，也不会证明调用者有该任务权限。

输入文件清单、基线、实际 Git OID、候选类型、目标 ref/OID 及上述全部绑定进入 `identity_digest`。`verify_current` 重新采集并比较完整对象，拒绝不同任务、环境声明、候选种类、文件内容、文件模式和目标身份。仅修改调用方声明值并不能证明真实工具链或配置得到核实。

Git loose／pack 对象的文件名或 `cat-file` 成功不足以证明内容身份。本库对实际读取的 commit、每层 tree 及 blob，以 Git 原始 `<type> <length>\0` 头加对象体重新计算 SHA1／SHA256 并比对 OID；先核验 tree，再解释原始目录项。内容被其他有效 zlib 对象替换时直接拒绝，不能把替换后的内容挂到旧 OID 下。

## 四种候选

| Selection | 实际来源 | 快照定位 |
|---|---|---|
| Worktree | `ls-files --cached --others -z`，包括被 `.gitignore` 忽略的实际文件；读取原始磁盘字节 | 当前 HEAD、显式 base、完整原始输入清单与差异摘要 |
| Index | 真实 stage 0 索引指向的 blob，复制索引后在私有对象目录计算树 | 当前 HEAD、显式 base、实际 index tree OID |
| Commit | 显式完整 commit OID 对应的真实 Git tree/blob | 明确 commit/base/tree，不使用当前未提交文件 |
| MergeCandidate | `merge-tree --write-tree expectedTarget head` 实际合并结果 | head、目标分支 ref、expectedTargetOid、实际合入树 OID |

不同候选不能复用证据；即使文件清单相同，类型及完整绑定仍区分它们。MergeCandidate 仅接受完整本地 `refs/heads/...`，读取当前目标必须等于固定 expectedTargetOid，漂移即拒绝。合并冲突返回错误，不把冲突输出当候选成功。未进行 fetch、checkout、更新调用方 ref 或修改调用方 index／工作树。

Git 的 write-tree 和 merge-tree 必须产生对象；本库将它们写入私有临时 `objects` 目录，读取原仓对象仅作为明确 alternate，并使用私有 index。临时目录清理后 tree OID 仍是内容身份，但计算生成的新树对象不持久化到调用方仓库。`capture` 本身不物化执行目录；新增 `materialize` 在相同私有对象仍存活时复制候选输入，后续 runner 必须使用该目录，不能直接拿工作树运行结果冒充 index／merge 验证。私有目录不是 OS 沙箱，也不是可写原仓 checkout。

## 路径、范围和拒绝项

- 路径以 Git 原始字节读取，使用协议 `GitPath` 的规范 base64，无大小写折叠、Unicode 归一化或有损 UTF-8 解码。rename 通过基线／候选的原始路径与 blob 身份保持可见，不使用相似度推断覆盖精确路径。
- Git 文件模式 `100644`、`100755` 被保留。symlink `120000`、gitlink/submodule `160000`、其他模式、unmerged/split index、worktree 的 skip-worktree／assume-unchanged 均明确拒绝，不宣称已支持。
- Unix 文件名可保留非法 UTF-8、tab、newline；Windows 无法表示的工作树名称、反斜杠／冒号别名直接拒绝。Git 元数据目录必须能被安全、无歧义地表示，不能通过有损转换读取。Windows executable 位沿用 Git index 模式，不宣称宿主提供 POSIX mode。
- Worktree 的每条原始路径必须与现有目录项的规范绝对路径精确一致；另维护规范路径集合拒绝多个名称映射同一路径。Windows 中 item.txt 实际文件不能被索引 ITEM.txt 别名冒充第二个输入；即使清单只保留错误大小写名称也拒绝。不按文件内容或物理文件 ID 去重，因此两个真实硬链接目录项仍分别保留。该检查仅用于 Worktree，Index/Commit 中合法的大小写不同 Git 路径继续保持原始字节与独立身份。重采集检测仍不是 OS 原子快照或恶意同用户竞态防护。
- Worktree 纳入 ignored 文件，避免 `.env` 或 ignored 配置被静默排除。大型构建输出也会被纳入并可能触发边界拒绝；本版不提供自动缩小范围的排除开关。空目录与 Git 元数据不是源码文件清单；外部依赖、环境变量、构建缓存及仓外文件真实性须由独立环境契约处理。
- 每文件／blob 最大 4 MiB，总输入最大 128 MiB，最多 4096 文件；Git inventory 输出 1 MiB、普通 blob 输出 4 MiB、stderr 64 KiB，捕获共享 60 秒截止时间。超限／缺对象／工具不支持／配置不安全均返回错误，没有弱化回退。

## 进程和并发边界

只执行固定 Git argv，不使用 shell，不执行报告或源码内命令。清除继承的所有 `GIT_*` 路由变量，禁系统／全局 Git config、对象 replace、lazy fetch、可选锁、fsmonitor 和自动维护。Git executable 来自调用端受信 PATH，不能把本库当作恶意 executable 的沙箱。

合并前枚举有效配置（含 include）；发现自定义 merge driver 或 clean/smudge/process filter 即拒绝，避免 merge renormalization 执行候选配置命令。工作树读取不调用 clean/smudge。不会执行用户提供的 Git 动作。

合并算法还可能读取祖先提交和 merge-base，因此合并前运行有界、只读的 `git fsck --strict --no-progress --no-reflogs --no-dangling`，并拒绝仓库自定义 `fsck.*` 弱化配置。损坏历史或对象不可用时不生成合入候选；此检查可能扫描更多对象，大型仓库超时仍明确失败。

捕获重复读取清单和内容，检测常见并发输入变化，但不是操作系统原子快照，不抵御同一权限的恶意文件替换、ABA 变化或篡改受信 Git executable。生产消费端应提供隔离、固定且受保护的 checkout；本版不声称已完成 EG-K06 进程树沙箱或 EG-K09 独立授权。

## 实际验证

```text
cargo test --locked --offline --manifest-path crates/guard-subject/Cargo.toml
cargo clippy --locked --offline --manifest-path crates/guard-subject/Cargo.toml --all-targets -- -D warnings
cargo +1.85 test --locked --offline --manifest-path crates/guard-subject/Cargo.toml
```

新增 Windows 工作树别名用例先实际观察 item.txt 与 ITEM.txt 被同时接受（RED），随后通过规范路径与原始名称精确比较拒绝；同时验证单个错误大小写名称拒绝、SHA1/SHA256 的 Index/Commit 双 Git 路径仍合法，以及真实硬链接两个目录项不被误拒（GREEN）。

Windows Git 2.55.0 的真实临时仓库测试覆盖 SHA1／SHA256、暂存区与 commit／worktree 区分、rename、mode、实际合入内容、目标漂移、合并冲突、symlink／submodule 拒绝、文件变更和体积限制。ignored 文件纳入清单测试已实际经历行为 RED（忽略配置不在清单）→修复→GREEN。另一个真实 RED（替换 loose blob 仍被旧 OID 接受）→GREEN 测试覆盖两种对象格式下 blob／tree／commit 的合法压缩对象替换。另有 Unix 专用非法 UTF-8／组合 Unicode 路径测试，未在 Windows 上执行，不据此宣称 Unix 平台已验收。需要支持 `merge-tree --write-tree` 的现代 Git（2.38+），SHA256 也由实际 Git 能力决定；不支持则明确失败。


EG-K04原任务范围的独立审查、Windows别名/真实硬链接focused运行及剩余平台边界见[局部验收证据](../../docs/delivery/EG-K04-evidence.md)。这不代表K06执行隔离或P0集成可交付。


## 私有候选物化

```rust,ignore
let materialized = guard_subject::materialize(
    project, &binding, &selection, &previous, scratch_parent,
)?;
materialized.verify_inputs()?;
let execution_root = materialized.path();
let same_subject = materialized.captured();
```

同步入口返回 `Result<MaterializedSubject, String>`，对象拥有 TempDir，提供 `path()`、`captured()`、`verify_inputs()` 三个借用方法，无 Deserialize；可 Send+Sync 及 Arc 共享。最后一个所有者 Drop 后尝试清理临时目录，宿主应在整个进程和结束复核期间持有对象。与 tempfile 通常语义一样，进程骤死无法承诺执行 Drop，拒绝的输入不会被当作有效物化对象返回。

scratch_parent 必须是显式提供、绝对且已存在的目录，其祖先无 symlink/reparse；不接受原项目及真实 gitdir、common-dir、objects 内部位置。先用 GitClient 的只读 inspect 与固定有界 argv 检查这些位置，完成后才创建临时目录，包含 linked worktree 的外部共享 Git 元数据也不能先写后拒绝。私有 Git index/objects 与 inputs 为并列目录，工具的工作目录只能采用 inputs。Unix 外层 TempDir 显式设为 0700；Windows 继承调用者配置的父目录 ACL，不声称独立验过 ACL。Git for Windows 的私有 index.lock 不接受 canonical 的 verbatim 前缀，因此仅在传给 Git 的私有绝对路径环境中转换等价 drive/UNC 表示，不改变任何原始 Git 文件名或候选摘要。

同一 GitClient 中前两次重新采集必须逐字段等于 previous，然后逐条写独立的新文件；复制完成再采集一次，发现已知来源或目标漂移即拒绝。Git Index/Commit/MergeCandidate 从真实对象中读取 blob，再核对 Git OID 和原始字节 contentDigest；合并计算生成的新 blob 仅存在于该次私有 objects，必须在目录销毁之前读取。Worktree 直接读取原路径，核对规范路径、类型、大小、摘要与适用模式。候选不因普通工作树脏字节而改变 Git 对象输入；两者分别代表各自实际字节。

目标文件只以 create_new 创建，不用原仓硬链接，也不覆盖临时目录内已有名称。父目录逐级创建并核对路径，拒绝 symlink/reparse 和大小写/规范路径别名。所有层级的 `.git`、大小写/尾点空格别名以及 `git~1` 等控制路径保守拒绝，不能复制指向原 repo 的 gitdir 文件。手工构造的 Git tree 也适用。Windows 无法同时物化的大小写不同 Git 路径明确拒绝；原 Index/Commit 捕获仍能无损保留这些合法 Git 路径。Unix 文件实际设置 100644/100755 对应权限并复验 executable 位，Windows 保留冻结 Git mode 声明，不伪称 POSIX executable 位已设置。

`verify_inputs` 只复验冻结的每条原始输入仍存在、名称精确、类型为普通文件、原始字节和适用 mode 一致；修改、删除或大小写别名替换均失败。合法新增构建输出允许存在，但不会进入 captured manifest，不成为新批准输入或质量通过证据。原输入字节检查不是 OS 原子快照，不抵御恶意同用户 ABA/任意并发篡改。此对象也不约束工具 argv、环境、文件或网络权限；宿主必须避免参数偷偷指回原 repo，并处理 Git 向执行根祖先发现仓库的问题（例如经明确 pin 的 ceiling 环境）。这些边界不得通过物化目录存在而冒认为已建立 OS 沙箱。

真实 TDD 先观察缺少 materialize 公共 API 的编译失败；随后在 Windows 的 Index 物化实际遇到 verbatim 私有 index.lock 创建失败，再修复环境路径表示并复验。新增7项 Windows行为测试覆盖 SHA1/SHA256 四种候选实际内容、仅私有 objects 中的合并新 blob、原 index/全部 refs/objects 保持不写、构建输出允许但不增输入、源/目标篡改及删除、无原仓硬链接、mode、Git控制路径原始 tree、linked worktree 外部元数据父目录、实际 junction 父目录、大小写别名，以及生命周期清理与 Send+Sync。当前Windows全包16项真实测试的最终结果以实际执行记录为准；Unix专用模式和无损路径分支尚未在本机执行，不据此宣称Unix平台验收。
