use crate::{
    AuditEvent, PublicSummary, RecordState, StoreLimits, WritePhase, filesystem, record::Record,
    store_lock::StoreLock,
};
use guard_protocol::{Digest, Identifier};
use rusqlite::{Connection, OpenFlags, OptionalExtension, TransactionBehavior, params};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

const APPLICATION_ID: i64 = 0x47535431;

/// 受管私有证据存储；调用者负责目录 ACL、可信时间与独立身份隔离。无 Java 对应类型。
pub struct EvidenceStore {
    root: PathBuf,
    limits: StoreLimits,
    lock_identity: String,
}
impl EvidenceStore {
    /// 创建全新私有目录和 SQLite；拒绝覆盖任何现有目录，不触碰旧 .codeguard。
    /// 参数必须为绝对目录及有界配额，返回尚无信任声明的存储实例。
    pub fn create(root: &Path, limits: StoreLimits) -> Result<Self, String> {
        limits.validate()?;
        if !root.is_absolute() || fs::symlink_metadata(root).is_ok() {
            return Err("new absolute store directory required".into());
        }
        filesystem::directory(root.parent().ok_or("store parent required")?)?;
        filesystem::mkdir(root)?;
        filesystem::mkdir(&root.join("artifacts"))?;
        filesystem::write_new(&root.join("writer.lock"), b"")?;
        filesystem::write_new(&root.join("evidence.sqlite3"), b"")?;
        let store = Self {
            root: filesystem::directory(root)?,
            limits,
            lock_identity: filesystem::identity(&root.join("writer.lock"))?,
        };
        let _lock = store.lock()?;
        let connection = store.connect(false)?;
        connection.execute_batch(&format!("BEGIN IMMEDIATE;
            PRAGMA application_id={APPLICATION_ID}; PRAGMA user_version=2;
            CREATE TABLE store_metadata (singleton INTEGER PRIMARY KEY CHECK(singleton=1), lock_identity TEXT NOT NULL);
            CREATE TABLE records (
                operation TEXT PRIMARY KEY NOT NULL,
                digest TEXT NOT NULL,
                size INTEGER NOT NULL CHECK(size >= 0),
                retain_until INTEGER NOT NULL CHECK(retain_until > 0),
                state TEXT NOT NULL CHECK(state IN ('pending','complete','quarantined','expired'))
            );
            CREATE TABLE audit (
                sequence INTEGER PRIMARY KEY AUTOINCREMENT,
                operation TEXT NOT NULL REFERENCES records(operation),
                code TEXT NOT NULL CHECK(code IN ('intent','completed','incomplete_recovered','artifact_invalid','expired')),
                at_ms INTEGER NOT NULL CHECK(at_ms >= 0)
            );
            CREATE TABLE heads (
                key TEXT PRIMARY KEY NOT NULL,
                revision INTEGER NOT NULL CHECK(revision>0),
                operation TEXT NOT NULL REFERENCES records(operation),
                digest TEXT NOT NULL,
                previous_revision INTEGER,
                previous_operation TEXT,
                previous_digest TEXT,
                CHECK ((previous_revision IS NULL AND previous_operation IS NULL AND previous_digest IS NULL)
                    OR (previous_revision>0 AND previous_operation IS NOT NULL AND previous_digest IS NOT NULL))
            );
            CREATE TABLE head_audit (
                sequence INTEGER PRIMARY KEY AUTOINCREMENT,
                key TEXT NOT NULL,
                operation TEXT NOT NULL REFERENCES records(operation),
                code TEXT NOT NULL CHECK(code IN ('applied','conflict')),
                at_ms INTEGER NOT NULL CHECK(at_ms>=0)
            ); COMMIT;")).map_err(|e| e.to_string())?;
        connection
            .execute(
                "INSERT INTO store_metadata(singleton,lock_identity) VALUES(1,?1)",
                [&store.lock_identity],
            )
            .map_err(|e| e.to_string())?;
        filesystem::sync_directory(&store.root)?;
        Ok(store)
    }

    /// 打开已有数据库，校验版本/完整性并隔离中断或损坏记录；不重建损坏数据库。
    /// 参数为受管绝对目录和限制，恢复不会产生执行批准或修改原始审计。
    pub fn open(root: &Path, limits: StoreLimits) -> Result<Self, String> {
        limits.validate()?;
        let store = Self {
            root: filesystem::directory(root)?,
            limits,
            lock_identity: filesystem::identity(&root.join("writer.lock"))?,
        };
        let _lock = store.lock()?;
        let mut connection = store.connect(true)?;
        store.recover_locked(&mut connection)?;
        Ok(store)
    }

    fn check_paths(&self) -> Result<(), String> {
        filesystem::managed_directory(&self.root)?;
        filesystem::managed_directory(&self.root.join("artifacts"))?;
        for name in [
            "writer.lock",
            "evidence.sqlite3",
            "evidence.sqlite3-wal",
            "evidence.sqlite3-shm",
            "evidence.sqlite3-journal",
        ] {
            let path = self.root.join(name);
            match fs::symlink_metadata(&path) {
                Ok(_) => filesystem::regular(&path)?,
                Err(error)
                    if error.kind() == std::io::ErrorKind::NotFound
                        && name != "writer.lock"
                        && name != "evidence.sqlite3" => {}
                Err(error) => return Err(error.to_string()),
            }
        }
        if filesystem::identity(&self.root.join("writer.lock"))? != self.lock_identity {
            return Err("writer lock identity changed".into());
        }
        Ok(())
    }

    fn lock(&self) -> Result<StoreLock, String> {
        self.check_paths()?;
        let lock = StoreLock::acquire(&self.root.join("writer.lock"), self.limits.busy_timeout_ms)?;
        self.check_paths()?;
        Ok(lock)
    }

    fn connect(&self, existing: bool) -> Result<Connection, String> {
        self.check_paths()?;
        let connection = Connection::open_with_flags(
            self.root.join("evidence.sqlite3"),
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .map_err(|e| e.to_string())?;
        connection
            .busy_timeout(Duration::from_millis(self.limits.busy_timeout_ms))
            .map_err(|e| e.to_string())?;
        if existing {
            let application: i64 = connection
                .query_row("PRAGMA application_id", [], |row| row.get(0))
                .map_err(|e| e.to_string())?;
            let version: i64 = connection
                .query_row("PRAGMA user_version", [], |row| row.get(0))
                .map_err(|e| e.to_string())?;
            if application != APPLICATION_ID || version != 2 {
                return Err("unsupported store database identity/version".into());
            }
            let expected_lock: String = connection
                .query_row(
                    "SELECT lock_identity FROM store_metadata WHERE singleton=1",
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;
            if expected_lock != self.lock_identity {
                return Err("persistent writer lock identity differs".into());
            }
            let check: String = connection
                .query_row("PRAGMA quick_check(1)", [], |row| row.get(0))
                .map_err(|e| e.to_string())?;
            if check != "ok" {
                return Err(
                    "store database integrity failed; preserve for external recovery".into(),
                );
            }
        }
        connection.execute_batch("PRAGMA trusted_schema=OFF; PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;").map_err(|e| e.to_string())?;
        self.check_paths()?;
        Ok(connection)
    }

    fn operation(operation: &str) -> Result<String, String> {
        Identifier::new(operation)?;
        Ok(filesystem::digest(operation.as_bytes()))
    }

    fn record(connection: &Connection, operation: &str) -> Result<Option<Record>, String> {
        let row = connection
            .query_row(
                "SELECT operation,digest,size,retain_until,state FROM records WHERE operation=?1",
                [operation],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, u64>(2)?,
                        row.get::<_, u64>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| e.to_string())?;
        row.map(|(operation, digest, size, retain_until, state)| {
            let record = Record {
                operation,
                digest,
                size,
                retain_until,
                state: RecordState::parse(&state)?,
            };
            record.summary()?;
            Ok(record)
        })
        .transpose()
    }

    fn records(&self, connection: &Connection) -> Result<Vec<Record>, String> {
        let mut query = connection
            .prepare("SELECT operation FROM records ORDER BY operation LIMIT ?1")
            .map_err(|e| e.to_string())?;
        let rows = query
            .query_map([self.limits.max_records + 1], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        let mut records = Vec::new();
        for row in rows {
            records.push(
                Self::record(connection, &row.map_err(|e| e.to_string())?)?
                    .ok_or("record disappeared")?,
            );
            if records.len() as u64 > self.limits.max_records {
                return Err("database record quota exceeded".into());
            }
        }
        Ok(records)
    }

    fn artifact(&self, record: &Record) -> Result<PathBuf, String> {
        Ok(self.root.join("artifacts").join(record.filename()?))
    }

    fn verify_artifact(&self, record: &Record) -> Result<Vec<u8>, String> {
        let data = filesystem::read(&self.artifact(record)?, self.limits.max_artifact_bytes)?;
        if data.len() as u64 != record.size || filesystem::digest(&data) != record.digest {
            return Err("private artifact length/digest mismatch".into());
        }
        Ok(data)
    }

    fn event(
        connection: &Connection,
        operation: &str,
        code: &str,
        now_ms: u64,
    ) -> Result<(), String> {
        connection
            .execute(
                "INSERT INTO audit(operation,code,at_ms) VALUES(?1,?2,?3)",
                params![operation, code, now_ms],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn quarantine(connection: &mut Connection, record: &Record, code: &str) -> Result<(), String> {
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE records SET state='quarantined' WHERE operation=?1",
            [&record.operation],
        )
        .map_err(|e| e.to_string())?;
        Self::event(&tx, &record.operation, code, 0)?;
        tx.commit().map_err(|e| e.to_string())
    }

    fn recover_locked(&self, connection: &mut Connection) -> Result<(), String> {
        for record in self.records(connection)? {
            if record.state == RecordState::Pending {
                Self::quarantine(connection, &record, "incomplete_recovered")?;
            } else if record.state == RecordState::Complete
                && self.verify_artifact(&record).is_err()
            {
                Self::quarantine(connection, &record, "artifact_invalid")?;
            }
        }
        Ok(())
    }

    /// 记录不可变操作及原件；相同操作 ID/字节/保留期幂等返回，内容变化拒绝覆盖。
    /// `now_ms` 与 `retain_until_ms` 仅为调用者时间，不产生可信时间或批准声明。
    pub fn put(
        &self,
        operation: &str,
        bytes: &[u8],
        retain_until_ms: u64,
        now_ms: u64,
    ) -> Result<PublicSummary, String> {
        self.put_observed(operation, bytes, retain_until_ms, now_ms, |_| {})
    }

    /// 按持久化阶段通知观察者；用于运行监控或故障注入，不允许观察者改变成功状态。
    /// 回调 panic/进程中止后，已提交 pending 意图会在下次打开时隔离。
    pub fn put_observed(
        &self,
        operation: &str,
        bytes: &[u8],
        retain_until_ms: u64,
        now_ms: u64,
        mut observer: impl FnMut(WritePhase),
    ) -> Result<PublicSummary, String> {
        let operation = Self::operation(operation)?;
        if bytes.len() as u64 > self.limits.max_artifact_bytes
            || now_ms > i64::MAX as u64
            || retain_until_ms > i64::MAX as u64
            || retain_until_ms <= now_ms
            || retain_until_ms - now_ms > self.limits.max_retention_ms
        {
            return Err("artifact or retention outside limits".into());
        }
        let _lock = self.lock()?;
        let mut connection = self.connect(true)?;
        let record = Record {
            operation,
            digest: filesystem::digest(bytes),
            size: bytes.len() as u64,
            retain_until: retain_until_ms,
            state: RecordState::Complete,
        };
        if let Some(existing) = Self::record(&connection, &record.operation)? {
            if existing.state != RecordState::Complete
                || existing.digest != record.digest
                || existing.size != record.size
                || existing.retain_until != record.retain_until
            {
                return Err(
                    "operation already recorded with different content/retention or unusable state"
                        .into(),
                );
            }
            if self.verify_artifact(&existing).is_err() {
                Self::quarantine(&mut connection, &existing, "artifact_invalid")?;
                return Err("existing artifact invalid; operation quarantined".into());
            }
            return existing.summary();
        }
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let records = self.records(&tx)?;
        if records.len() as u64 >= self.limits.max_records {
            return Err("record quota reached; audit not deleted".into());
        }
        let mut total = 0u64;
        let mut count = 0;
        for entry in fs::read_dir(self.root.join("artifacts")).map_err(|e| e.to_string())? {
            count += 1;
            if count > self.limits.max_records {
                return Err("artifact inventory quota exceeded".into());
            }
            let path = entry.map_err(|e| e.to_string())?.path();
            filesystem::regular(&path)?;
            total = total
                .checked_add(fs::metadata(&path).map_err(|e| e.to_string())?.len())
                .ok_or("artifact quota overflow")?;
        }
        let reserved = records
            .iter()
            .filter(|item| item.state != RecordState::Expired)
            .try_fold(0u64, |sum, item| {
                sum.checked_add(item.size)
                    .ok_or("reservation quota overflow")
            })?;
        if total
            .max(reserved)
            .checked_add(record.size)
            .ok_or("artifact quota overflow")?
            > self.limits.max_total_bytes
        {
            return Err("private byte quota reached".into());
        }
        tx.execute("INSERT INTO records(operation,digest,size,retain_until,state) VALUES(?1,?2,?3,?4,'pending')",
            params![record.operation,record.digest,record.size,record.retain_until]).map_err(|e| e.to_string())?;
        Self::event(&tx, &record.operation, "intent", now_ms)?;
        tx.commit().map_err(|e| e.to_string())?;
        observer(WritePhase::IntentDurable);
        self.check_paths()?;
        filesystem::write_new(&self.artifact(&record)?, bytes)?;
        observer(WritePhase::ArtifactDurable);
        self.check_paths()?;
        self.verify_artifact(&record)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let changed = tx
            .execute(
                "UPDATE records SET state='complete' WHERE operation=?1 AND state='pending'",
                [&record.operation],
            )
            .map_err(|e| e.to_string())?;
        if changed != 1 {
            return Err("pending record changed during write".into());
        }
        Self::event(&tx, &record.operation, "completed", now_ms)?;
        tx.commit().map_err(|e| e.to_string())?;
        observer(WritePhase::Complete);
        record.summary()
    }

    /// 读取私有原文；每次核验原件摘要，缺失/损坏时事务隔离并拒绝返回。
    /// 调用者应在受保护身份内使用，目录访问权不等于远程主体授权。
    pub fn read_private(&self, operation: &str, now_ms: u64) -> Result<Vec<u8>, String> {
        let _lock = self.lock()?;
        let mut connection = self.connect(true)?;
        let record =
            Self::record(&connection, &Self::operation(operation)?)?.ok_or("unknown operation")?;
        if record.state != RecordState::Complete || now_ms >= record.retain_until {
            return Err("operation incomplete, quarantined or expired".into());
        }
        match self.verify_artifact(&record) {
            Ok(bytes) => Ok(bytes),
            Err(_) => {
                Self::quarantine(&mut connection, &record, "artifact_invalid")?;
                Err("artifact invalid; isolated for recovery".into())
            }
        }
    }

    /// 查询本地状态，不把 Complete 转为信任或批准。
    pub fn state(&self, operation: &str) -> Result<RecordState, String> {
        let _lock = self.lock()?;
        let connection = self.connect(true)?;
        Ok(Self::record(&connection, &Self::operation(operation)?)?
            .ok_or("unknown operation")?
            .state)
    }

    /// 返回固定白名单公开摘要；摘要不能替代私有原件检查。
    pub fn summary(&self, operation: &str) -> Result<PublicSummary, String> {
        let _lock = self.lock()?;
        let connection = self.connect(true)?;
        Self::record(&connection, &Self::operation(operation)?)?
            .ok_or("unknown operation")?
            .summary()
    }

    /// 返回受管原件位置供本地诊断；不创建路径、不跟随链接、不授予原件读取权。
    pub fn artifact_path(&self, operation: &str) -> Result<PathBuf, String> {
        let _lock = self.lock()?;
        let connection = self.connect(true)?;
        self.artifact(
            &Self::record(&connection, &Self::operation(operation)?)?.ok_or("unknown operation")?,
        )
    }

    /// 对完整、pending 和缺件记录执行恢复校验，返回后仍不产生工程通过声明。
    pub fn recover(&self) -> Result<(), String> {
        let _lock = self.lock()?;
        let mut connection = self.connect(true)?;
        self.recover_locked(&mut connection)
    }

    /// 先持久化失效状态和审计再删除到期原件；删除中断后可幂等重试。
    /// 元数据和原始异常审计保留，配额满时拒绝新写而非删除历史。
    pub fn expire(&self, now_ms: u64) -> Result<usize, String> {
        if now_ms > i64::MAX as u64 {
            return Err("invalid retention clock".into());
        }
        let _lock = self.lock()?;
        let mut connection = self.connect(true)?;
        let mut expired = 0;
        for record in self.records(&connection)? {
            if record.retain_until > now_ms {
                continue;
            }
            if record.state != RecordState::Expired {
                let tx = connection
                    .transaction_with_behavior(TransactionBehavior::Immediate)
                    .map_err(|e| e.to_string())?;
                tx.execute(
                    "UPDATE records SET state='expired' WHERE operation=?1",
                    [&record.operation],
                )
                .map_err(|e| e.to_string())?;
                Self::event(&tx, &record.operation, "expired", now_ms)?;
                tx.commit().map_err(|e| e.to_string())?;
                expired += 1;
            }
            let path = self.artifact(&record)?;
            match fs::symlink_metadata(&path) {
                Ok(_) => {
                    filesystem::regular(&path)?;
                    fs::remove_file(path).map_err(|e| e.to_string())?;
                    filesystem::sync_directory(&self.root.join("artifacts"))?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.to_string()),
            }
        }
        Ok(expired)
    }

    /// 读取追加审计白名单；不会返回私有原文、未脱敏异常消息或原始操作 ID。
    pub fn audit_events(&self) -> Result<Vec<AuditEvent>, String> {
        let _lock = self.lock()?;
        let connection = self.connect(true)?;
        let mut query = connection
            .prepare("SELECT sequence,operation,code,at_ms FROM audit ORDER BY sequence LIMIT ?1")
            .map_err(|e| e.to_string())?;
        let rows = query
            .query_map([self.limits.max_records * 4 + 1], |row| {
                Ok((
                    row.get::<_, u64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, u64>(3)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut events = Vec::new();
        for row in rows {
            let (sequence, operation, code, at_ms) = row.map_err(|e| e.to_string())?;
            if ![
                "intent",
                "completed",
                "incomplete_recovered",
                "artifact_invalid",
                "expired",
            ]
            .contains(&code.as_str())
            {
                return Err("invalid audit event code".into());
            }
            events.push(AuditEvent {
                sequence,
                operation_digest: Digest::new(operation)?,
                code,
                at_ms,
            });
            if events.len() as u64 > self.limits.max_records * 4 {
                return Err("audit quota exceeded".into());
            }
        }
        Ok(events)
    }
}

impl EvidenceStore {
    fn head_entry(
        connection: &Connection,
        key: &str,
    ) -> Result<Option<crate::head_entry::HeadEntry>, String> {
        let raw = connection.query_row(
            "SELECT revision,operation,digest,previous_revision,previous_operation,previous_digest FROM heads WHERE key=?1",
            [key], |row| Ok((row.get::<_,u64>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,
                row.get::<_,Option<u64>>(3)?,row.get::<_,Option<String>>(4)?,row.get::<_,Option<String>>(5)?)))
            .optional().map_err(|e| e.to_string())?;
        raw.map(
            |(
                revision,
                operation,
                digest,
                previous_revision,
                previous_operation,
                previous_digest,
            )| {
                if revision == 0 {
                    return Err("invalid head revision".into());
                }
                let identity = crate::HeadIdentity {
                    key_digest: Digest::new(key.to_owned())?,
                    revision,
                    operation_digest: Digest::new(operation)?,
                    content_digest: Digest::new(digest)?,
                };
                let previous = match (previous_revision, previous_operation, previous_digest) {
                    (None, None, None) if revision == 1 => None,
                    (Some(old), Some(operation), Some(digest))
                        if old.checked_add(1) == Some(revision) =>
                    {
                        Some(crate::HeadIdentity {
                            key_digest: identity.key_digest.clone(),
                            revision: old,
                            operation_digest: Digest::new(operation)?,
                            content_digest: Digest::new(digest)?,
                        })
                    }
                    _ => return Err("invalid head predecessor".into()),
                };
                Ok(crate::head_entry::HeadEntry { identity, previous })
            },
        )
        .transpose()
    }

    // 错误中的bool仅表示隔离与审计均已写入当前事务，调用方必须提交该隔离。
    fn verify_head_artifact(
        &self,
        connection: &Connection,
        record: &Record,
    ) -> Result<Vec<u8>, (String, bool)> {
        if let Ok(bytes) = self.verify_artifact(record) {
            return Ok(bytes);
        }
        let isolated = (|| -> Result<(), String> {
            connection.execute("UPDATE records SET state='quarantined' WHERE operation=?1 AND state='complete'",[&record.operation]).map_err(|e|e.to_string())?;
            Self::event(connection, &record.operation, "artifact_invalid", 0)?;
            Ok(())
        })();
        match isolated {
            Ok(()) => Err(("head artifact invalid; operation quarantined".into(), true)),
            Err(error) => Err((error, false)),
        }
    }

    fn head_bytes(
        &self,
        connection: &Connection,
        identity: &crate::HeadIdentity,
        now_ms: u64,
    ) -> Result<Vec<u8>, (String, bool)> {
        let record = Self::record(connection, identity.operation_digest.as_str())
            .map_err(|e| (e, false))?
            .ok_or_else(|| ("head operation missing".to_string(), false))?;
        if record.state != RecordState::Complete
            || now_ms >= record.retain_until
            || record.digest != identity.content_digest.as_str()
        {
            return Err((
                "head record incomplete, quarantined, expired or identity mismatch".into(),
                false,
            ));
        }
        self.verify_head_artifact(connection, &record)
    }

    fn head_event(
        &self,
        connection: &Connection,
        key: &str,
        operation: &str,
        code: &str,
        now_ms: u64,
    ) -> Result<(), String> {
        let count: u64 = connection
            .query_row("SELECT COUNT(*) FROM head_audit", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if count >= self.limits.max_records * 4 {
            return Err("head audit quota reached; history not deleted".into());
        }
        connection
            .execute(
                "INSERT INTO head_audit(key,operation,code,at_ms) VALUES(?1,?2,?3,?4)",
                params![key, operation, code, now_ms],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// 在跨进程锁及同一SQLite事务内读取活跃头和私有原件；缺件或失效时拒绝返回。
    /// 参数为不透明头名称和调用者时间；返回值不表示领域批准或执行授权。
    pub fn read_head(&self, key: &str, now_ms: u64) -> Result<Option<crate::HeadRecord>, String> {
        if now_ms > i64::MAX as u64 {
            return Err("invalid retention clock".into());
        }
        let key = Self::operation(key)?;
        let _lock = self.lock()?;
        let mut connection = self.connect(true)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let result = match Self::head_entry(&tx, &key)? {
            None => None,
            Some(entry) => match self.head_bytes(&tx, &entry.identity, now_ms) {
                Ok(bytes) => Some(crate::HeadRecord {
                    identity: entry.identity,
                    bytes,
                }),
                Err((error, persist_quarantine)) => {
                    if persist_quarantine {
                        tx.commit().map_err(|e| e.to_string())?;
                    }
                    return Err(error);
                }
            },
        };
        tx.commit().map_err(|e| e.to_string())?;
        Ok(result)
    }

    /// 对完整令牌执行事务CAS；新操作必须已完整持久化且当前原件与保留期有效。
    /// expected=None仅允许首次建立；完整令牌含修订号防ABA，失败只追加有界冲突审计。
    /// 相同请求重试幂等，不能跳过原件完整性核验；本方法不判断任务或批准语义。
    pub fn compare_and_swap_head(
        &self,
        key: &str,
        expected: Option<&crate::HeadIdentity>,
        new_operation: &str,
        now_ms: u64,
    ) -> Result<crate::HeadUpdate, String> {
        if now_ms > i64::MAX as u64 {
            return Err("invalid retention clock".into());
        }
        let key = Self::operation(key)?;
        let operation = Self::operation(new_operation)?;
        let _lock = self.lock()?;
        let mut connection = self.connect(true)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| e.to_string())?;
        let record = Self::record(&tx, &operation)?.ok_or("new head operation missing")?;
        if record.state != RecordState::Complete || now_ms >= record.retain_until {
            return Err("new head operation incomplete, quarantined or expired".into());
        }
        if let Err((error, persist_quarantine)) = self.verify_head_artifact(&tx, &record) {
            if persist_quarantine {
                tx.commit().map_err(|e| e.to_string())?;
            }
            return Err(error);
        }
        let current = Self::head_entry(&tx, &key)?;
        if let Some(entry) = &current {
            if let Err((error, persist_quarantine)) = self.head_bytes(&tx, &entry.identity, now_ms)
            {
                if persist_quarantine {
                    tx.commit().map_err(|e| e.to_string())?;
                }
                return Err(error);
            }
        }
        // 仅相同新操作且expected确为当前或其直接前驱的请求可以幂等返回。
        if let Some(entry) = &current {
            if entry.identity.operation_digest.as_str() == operation
                && (expected == Some(&entry.identity) || expected == entry.previous.as_ref())
            {
                let identity = entry.identity.clone();
                tx.commit().map_err(|e| e.to_string())?;
                return Ok(crate::HeadUpdate::Unchanged(identity));
            }
        }
        let current_identity = current.as_ref().map(|entry| &entry.identity);
        if expected != current_identity {
            self.head_event(&tx, &key, &operation, "conflict", now_ms)?;
            let identity = current_identity.cloned();
            tx.commit().map_err(|e| e.to_string())?;
            return Ok(crate::HeadUpdate::Conflict(identity));
        }
        if current.is_none() {
            let count: u64 = tx
                .query_row("SELECT COUNT(*) FROM heads", [], |r| r.get(0))
                .map_err(|e| e.to_string())?;
            if count >= self.limits.max_records {
                return Err("head quota reached; identity not deleted".into());
            }
        }
        let revision = current_identity.map_or(Ok(1), |old| {
            old.revision
                .checked_add(1)
                .filter(|n| *n <= i64::MAX as u64)
                .ok_or("head revision exhausted")
        })?;
        self.head_event(&tx, &key, &operation, "applied", now_ms)?;
        tx.execute("INSERT INTO heads(key,revision,operation,digest,previous_revision,previous_operation,previous_digest) VALUES(?1,?2,?3,?4,?5,?6,?7)
            ON CONFLICT(key) DO UPDATE SET revision=excluded.revision,operation=excluded.operation,digest=excluded.digest,previous_revision=excluded.previous_revision,previous_operation=excluded.previous_operation,previous_digest=excluded.previous_digest",
            params![key,revision,operation,record.digest,current_identity.map(|i|i.revision),current_identity.map(|i|i.operation_digest.as_str()),current_identity.map(|i|i.content_digest.as_str())]).map_err(|e|e.to_string())?;
        let identity = crate::HeadIdentity {
            key_digest: Digest::new(key)?,
            revision,
            operation_digest: Digest::new(operation)?,
            content_digest: Digest::new(record.digest)?,
        };
        tx.commit().map_err(|e| e.to_string())?;
        Ok(crate::HeadUpdate::Applied(identity))
    }

    /// 读取活跃头固定代码审计，最多max_records*4条；不包含头原名或私有原件。
    pub fn head_audit_events(&self) -> Result<Vec<crate::HeadAuditEvent>, String> {
        let _lock = self.lock()?;
        let connection = self.connect(true)?;
        let mut query=connection.prepare("SELECT sequence,key,operation,code,at_ms FROM head_audit ORDER BY sequence LIMIT ?1").map_err(|e|e.to_string())?;
        let rows = query
            .query_map([self.limits.max_records * 4 + 1], |row| {
                Ok((
                    row.get::<_, u64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, u64>(4)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut events = Vec::new();
        for row in rows {
            let (sequence, key, operation, code, at_ms) = row.map_err(|e| e.to_string())?;
            if !["applied", "conflict"].contains(&code.as_str()) {
                return Err("invalid head audit code".into());
            }
            events.push(crate::HeadAuditEvent {
                sequence,
                key_digest: Digest::new(key)?,
                operation_digest: Digest::new(operation)?,
                code,
                at_ms,
            });
            if events.len() as u64 > self.limits.max_records * 4 {
                return Err("head audit quota exceeded".into());
            }
        }
        Ok(events)
    }
}
