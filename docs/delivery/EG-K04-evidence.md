# EG-K04 Git候选输入局部验收证据

日期：2026-10-17。原任务：实现SubjectSnapshot/index/commit/merge_candidate输入清单与无损路径身份；真实临时Git验证SHA1/SHA256、rename、mode、symlink/submodule支持或明确拒绝。

| 原任务义务 | 源码与实际行为 | 结论 |
|---|---|---|
| 实际候选清单 | guard-subject/src/capture.rs及manifest.rs从真实worktree/index/commit读取；merge-tree在私有对象目录计算实际merge_candidate，完整base/head/tree/ref/OID入身份 | 满足 |
| 显式身份绑定 | repository/task/worktree及contract/policy/verifier/environment声明与候选身份、实际输入清单绑定；verify_current完整重采集，不凭路径摘要猜repo身份 | 满足 |
| SHA1/SHA256及对象完整性 | 真实两种格式仓库；commit/tree/blob按原始Git对象头+字节重算OID。用另一有效zlib对象替换旧OID文件的6种组合拒绝 | 满足 |
| 原始路径、rename及mode | GitPath base64无损；不casefold/Unicode归一化；rename保持路径与对象身份，保留100644/100755 | 满足 |
| 特殊模式明确拒绝 | symlink120000、gitlink/submodule160000、unmerged/split index及worktree跳过标记明确拒绝，不假报支持 | 满足 |
| 不改调用方仓库 | 生产仅固定Git argv；无shell/fetch/checkout/ref更新；私有index/objects用于write-tree/merge-tree，不改调用方refs/index/worktree | 满足 |
| 输入边界 | 每件4MiB/总128MiB/4096文件与有界Git输出/共享截止时间；ignored配置也纳入，目标OID漂移、冲突、缺件和不安全配置拒绝 | 满足 |

## 必要反例与独立复验

已有真实RED→GREEN：ignored配置原本漏出清单；合法压缩blob被替换仍以旧OID读取；Windows单物理item.txt被item.txt/ITEM.txt索引别名重复接受。均先实际失败，再修复复验。

Windows修复只在Worktree读取检查原始名称与规范实际目录项精确一致，并维护规范路径集合；没有把大小写不同的Git对象路径合并，也不按物理文件ID去重硬链接。独立只读检查manifest.rs/paths.rs及tests/real_git.rs，并在Rust1.85独立goal-subject目标实际执行：

- `worktree_case_aliases_are_rejected_but_git_object_paths_remain_distinct`：1/1通过18.85s，覆盖SHA1/SHA256双别名、单错误大小写别名拒绝及Index/Commit两个原始名称保留。
- `distinct_real_hard_link_names_remain_valid_worktree_inputs`：1/1通过3.57s，真实两个目录项保留相同内容摘要，不误拒硬链接。

作者在union调整前后两次Rust1.85全套9项通过，并通过fmt/clippy；先前独立7项真实Git回归通过。新27项协议、23项contract、12项plan受影响回归亦通过；plan明确拒绝未支持的release捕获，不能用新协议类型伪装实际provider。

复验：`cargo +1.85 test --locked --offline --manifest-path crates/guard-subject/Cargo.toml`。TEMP/TMP、Cargo缓存和构建目标均在E盘。真实测试只操作自己临时仓库。

## 保证范围

当前仅四种Git候选采集，不提供release_artifact实际采集provider、可执行隔离checkout、受信repo注册表或生产授权。不声明OS原子快照或抵御同用户恶意竞态；现代Git能力不足明确失败。Unix非法UTF8/组合Unicode用例在当前Windows未运行，不据此宣称其他平台验收。最终执行/目标引用条件、进程隔离与跨平台证据仍归后续任务。K04局部完成不关闭EVI整条需求，不意味着P0集成可交付；K06仍未勾选。
