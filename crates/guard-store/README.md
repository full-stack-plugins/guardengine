# GuardStore：私有证据与恢复

本独立库实现 EG-K05 的本地 SQLite/文件存储基础，引用 `guard-protocol` 的 `Digest` 和 `Identifier`。它不签发批准、授权或可信生产者标签，也不修改历史 `.codeguard`。还没有接入 CLI、执行器或托管服务，不能把本库测试等同总体 P0 交付。

## 存储行为

`EvidenceStore::create(absolute_path, limits)` 只创建全新目录；`open` 只接受已有、版本匹配且 SQLite 完整性检查正常的存储，不会重新初始化坏数据库。

布局为 `evidence.sqlite3`、受管 WAL/SHM、`writer.lock`、`artifacts/<operation-sha256>-<content-sha256>.blob`。私有原文保持原始字节。操作 ID 仅以 SHA256 入库及公开摘要；公开摘要只有操作摘要、原文摘要、长度、本地状态，没有自动复制的自由文本、报告、日志或调用者描述。摘要仍可能暴露长度、相等性和可猜测内容的摘要，需依业务分类控制传播。

每次写入：

1. 获得有界跨进程协作锁，核对目录、数据库和 SQLite sidecar 路径。
2. 在 `BEGIN IMMEDIATE` 中核对记录数和原件字节配额（包括 pending/quarantined 预留），提交不可变 `pending` 意图与 `intent` 审计。
3. 以 `create_new` 创建原件，写入并 `sync_all`；Unix 还同步父目录。
4. 重新核验原件长度/摘要，在 SQLite 事务中把唯一 pending 转为 complete，同时追加 completed 审计。SQLite 使用 WAL 和 `synchronous=FULL`。

同一操作 ID、相同字节及保留期幂等返回；不同内容/保留期、不完整、隔离或已过期的操作不能覆盖或复活。摘要不是完整性检查的替代，消费证据须调用 `read_private`，它每次检查原件长度和摘要。

`open`/`recover` 将 pending、缺件和摘要错误的 complete 记录事务标为 quarantined，追加固定错误代码，不删除原记录、原文件或旧审计。隔离是数据库可用性隔离，不是把文件搬出目录。SQLite 自身通过 WAL 恢复未提交事务；数据库损坏则拒绝打开，保持原文件供受控外部恢复，不抹掉线索。

`expire(now_ms)` 先事务标记 expired 并追加审计，再删除到期原件；文件删除失败保持 expired，重试会继续删除而不重复审计。调用者必须安排该维护操作；打开存储不会擅自采用系统时间清理。调用者提供的时钟不是受信运行序号。最大单件/总字节/记录数/最长保留期/锁等待均有界；审计与操作身份不会为腾空间被删除，达到记录上限后拒绝新写。到期元数据继续保留，需服务层制定受控分区归档策略。

## 路径与身份边界

拒绝链接/reparse 祖先和管理目录，拒绝数据库、WAL、SHM、journal、锁和原件的符号链接、reparse point 及硬链接。SQLite 使用 `SQLITE_OPEN_NOFOLLOW`。Unix 管理目录为 0700、文件为 0600，打开现有宽权限目录/文件会拒绝。Windows 继承父目录 ACL，调用方必须预先设置私有 ACL；本库不声称 ACL 已被独立核验。

writer.lock 的文件身份（Unix dev/ino；Windows volume/file index）持久化在 SQLite，并在每次获取前后和打开数据库时匹配，普通文件替换不能静默成为新锁。替换后应由受控恢复者保留线索并恢复正确文件，不自动换锁消除异常。

本库假设管理目录由存储身份独占，不能抵御恶意同用户进程改写数据库、调换祖先目录或在检查与使用之间任意竞态。路径检查、文件摘要和协作锁不是 OS 沙箱或远程身份认证。生产隔离还需 K09/K10 的独立控制端、受保护目录与凭据。进程中止用例不代表断电/磁盘缓存故障验收；Windows 没有在本库执行目录 flush，磁盘与硬件掉电语义尚未验证。

## 依赖与验证

固定 `rusqlite = 0.37.0`（bundled SQLite 3.50.2）、`fs2 = 0.4.3`，使用本目录 Cargo.lock。官方来源：[rusqlite 0.37.0 API](https://docs.rs/rusqlite/0.37.0/rusqlite/)、[SQLite WAL](https://sqlite.org/wal.html)。已在当前 Windows x86_64 上用 Rust **1.85** 实际编译测试，不将其替代 Unix 平台验收。SQLite 引擎更新与服务端安全评估应在后续发布前单独核验。

```powershell
cargo +1.85 test --locked --offline
cargo +1.85 clippy --locked --offline --all-targets -- -D warnings
cargo +1.85 fmt --check
```

`tests/storage.rs` 包含真实 SQLite busy、并发连接、幂等、缺件/损坏、数据库损坏拒绝、配额、保留失效、硬链接/WAL、普通锁替换和 Windows junction。进程中断使用独立测试子进程在意图提交、部分原件、完整原件以及未提交 SQLite 事务后直接退出，不依赖模拟数据库。两个 worker 测试在普通单独运行时是入口辅助，不是额外产品验收场景。没有符号链接权限的 Windows 用例使用实际硬链接/junction 检查替换拒绝，不把未运行的 symlink 当通过。

初次 RED 是缺失公共 API 的编译失败；其后的真实 Windows 并发错误 33、硬链接接受、普通 writer.lock 替换三个行为反例曾失败，修复后通过。EG-K05原任务的本地存储范围已通过真实SQLite/文件/子进程验收及独立审查，见[验收证据](../../docs/delivery/EG-K05-evidence.md)。产品集成和其他平台证据仍归后续任务，不由本局部完成推断。

## 活跃头：通用跨进程 CAS（schema 2）

`put` 仍只创建不可变私有操作，不能使它自动成为活跃基线。本包新增通用 `heads` 指针与独立 `head_audit` 表；它不理解 TaskBinding、契约批准或任务迁移，也不需要全局生产 authority。领域调用者必须先完成自己的独立批准和校验，再调用这些机械端口：

```rust,ignore
read_head(key: &str, now_ms: u64) -> Result<Option<HeadRecord>, String>
compare_and_swap_head(
    key: &str,
    expected: Option<&HeadIdentity>,
    new_operation: &str,
    now_ms: u64,
) -> Result<HeadUpdate, String>
```

`HeadIdentity` 包含 key_digest、revision、operation_digest、content_digest。revision 从1递增，不删除或复用头身份；完整比较能拒绝 A→B→A 后过时的 A 令牌，也拒绝跨头、错误原件摘要或错误修订号。`HeadRecord` 在同一次跨进程锁和 SQLite 事务中返回核验后的身份与私有原始字节，调用者不必用第二次独立读取拼接一个可能过时的指针。

CAS 在原有 writer.lock 内执行 `BEGIN IMMEDIATE`，核对旧头和新操作都 Complete、尚未超过调用者提供的保留期、原件存在且长度/摘要正确，然后原子追加 applied 审计和更新指针。pending、quarantined、expired、缺件、损坏或不存在的操作不能成为活跃头。旧头损坏或失效时同样阻断更新，不自动清空指针或恢复资格；需要受控外部恢复或上层重新规划保留策略。`read_head` 和 CAS 观察到缺件、长度或摘要错误时，在原事务中持久化 quarantined 与 artifact_invalid 审计，提交隔离后才返回错误；不更新活跃头。即使随后恢复相同原始字节，隔离状态也不会复活。持久化隔离本身失败则回滚并拒绝返回，交由受控恢复；原有 `read_private`/recover/open 继续保留其隔离行为。

返回 `Applied(identity)` 表示本事务更新成功，`Conflict(current)` 表示完整 expected 不匹配且指针未改变。`Unchanged(identity)` 只适用于当前同一操作且 expected 等于当前身份或该修订的直接前驱（首次创建的前驱为 None），支持精确请求重试；幂等路径也重新核验原件、状态和保留期，不绕过安全边界。原始操作/字节保持不可变，即使头后来指向其他操作。

活跃头数量最多 max_records；独立头审计最多 max_records×4。达到配额拒绝新的创建、迁移或冲突记录，不删除历史腾空间；已成立的精确幂等重试不额外追加审计。冲突有固定 conflict 事件；更早的输入/原件校验失败直接返回错误，不添加无效操作的伪事件。公开头审计只包含本地序号、名称/操作摘要、固定代码及调用者时间，不公开头名称、私有字节或业务原因。调用者时间和存储 revision 都不是可信授权时间或运行序号。

数据库 user_version 从未发布候选1显式升级为2。`open` 对候选1或未知版本明确拒绝，不偷偷建表、重建数据库、复制数据或迁移原件。已有候选存储必须由受控迁移流程处理，本包没有自动迁移功能；不会因为本次增加活跃头而静默改写旧数据。

`tests/heads.rs` 增加9项行为验收及1个子进程入口：真实双进程 CAS 唯一胜者并重启验证、独立线程竞争、失败 CAS 不更新头、完整令牌防 ABA/跨头、幂等重试和普通 put 不产生活跃头、pending/隔离/缺件/损坏/到期拒绝、头与审计配额、旧schema拒绝且原记录保留。双进程通过实际 SQLite/协作锁运行，不用模拟存储。初始 RED 为缺失 CAS 公共 API 的实际编译失败。独立审查随后实证损坏原件恢复字节后会恢复活跃资格，新增行为测试先观察 Complete 而非 Quarantined 的失败，再修复为事务持久隔离；read/CAS旧头/CAS新头/幂等重试四条路径均覆盖，恢复原字节仍拒绝。随后在 Rust1.85 的真实数据库测试中复验实现。此处不声称提供领域批准、恶意同用户隔离、生产级可信控制器或断电验收。
