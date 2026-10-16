# EG-K05 本地证据存储局部验收证据

原任务：实现SQLite/文件证据存储和事务恢复；验证并发写、busy、落盘中断、链接替换、丢失artifact、脱敏和保留期限，不改旧.codeguard。

| 任务义务 | 源码与可观察证据 | 结论 |
|---|---|---|
| 持久化与恢复 | guard-store/tests/storage.rs真实pending意图、部分/完整原件后子进程中止；WAL未提交事务中止，重启隔离不完整记录，保留线索 | 满足 |
| 并发、busy、幂等 | 真实SQLite锁busy有界、线程同操作唯一结果，heads.rs双进程CAS唯一胜者，完整revision令牌阻止ABA | 满足 |
| 链接与替换 | 普通writer.lock身份持久核对、硬链接/WAL/junction/管理目录替换拒绝；数据库损坏不重建，丢失原件和摘要篡改隔离 | 满足 |
| 脱敏与retention | 私有原文原字节保存，公开摘要不包含token/password等自由文本；配额有界、到期删除原件保留审计/操作身份，不能重写复活 | 满足 |
| 不升级信任 | 本地状态、通用CAS不签发授权；新管理目录独立于.codeguard，不读写旧状态 | 满足 |

真实行为RED→GREEN包括Windows并发错误33、硬链接接受、普通锁替换。独立审查用真实SQLite证明损坏原件恢复相同字节后活跃资格复活；新增测试先观察Complete而非Quarantined失败，再把read/CAS-current/CAS-new/CAS-retry四路径改为事务持久隔离，恢复字节仍拒绝。

实际验证：Rust1.85独立goal-review-store全套27 reported tests通过，包含storage 17、heads 10；其中3个是子进程worker入口，不应计作27个独立验收场景。作者fmt/clippy通过。测试均为真实本地文件/SQLite/进程，未将符号链接权限缺失视为已运行；Windows采用实际硬链接/junction覆盖对应拒绝行为。

复验：`cargo +1.85 test --locked --offline --manifest-path crates/guard-store/Cargo.toml`。

保证范围为受保护独占目录内的本地协作进程。当前Windows实际验证不替代其他平台、断电/硬件缓存、恶意同用户隔离或K26托管存储验收；Windows未执行目录flush。调用者需控制ACL、时钟与唯一registry。

## 验证与边界

日期：2026-10-17。验收以原生 `codeguard/openspec/changes/add-engineering-guard-core/tasks.md` 的本任务字面为限；独立审查核对源码、真实行为反例与实际测试结果，未以任务勾选代替证明。Rust 1.85、固定 Cargo.lock、`--locked --offline`；TEMP/TMP、Cargo缓存与构建目标均在E盘。

EG-K01已按本地协议/字段清单冻结任务验收，仍保持candidate-unpublished；本库对未发布协议的局部实现已验证，不代表协议已经公开发布或P0产品集成可交付。CLI/MCP、真实受信授权提供者、OPA执行、有界runner、托管隔离与全链路验收仍由K06/K08/K09/K10/K11/K12及后续任务承担。不得据本任务关闭整条CON/EVI需求或P0路线；不sync/archive总体变更，不改既有alpha CLI或旧.codeguard状态。
