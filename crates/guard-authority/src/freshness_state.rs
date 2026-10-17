use guard_protocol::{Digest, UtcTimestamp};
use std::collections::{BTreeMap, BTreeSet};
/// 当前实例的时钟与撤销高水位；跨进程持久权威由独立source保证。无Java对应类型。
#[derive(Default)]
pub(crate) struct FreshnessState {
    pub(crate) now: Option<UtcTimestamp>,
    pub(crate) receipts: BTreeMap<String, Digest>,
    pub(crate) receipt_conflicts: BTreeSet<String>,
    pub(crate) revoked: BTreeSet<String>,
    pub(crate) snapshots: BTreeMap<String, (u64, Digest)>,
    pub(crate) records: BTreeMap<String, (u64, Digest)>,
}
