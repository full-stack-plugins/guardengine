use guard_protocol::{Digest, Identifier, UtcTimestamp};
use serde::Serialize;
/// 当前权威撤销状态；revision单调性跨重启由source保证。无Java对应类型。
#[derive(Clone, Debug, Serialize)]
pub struct RevocationSnapshot {
    pub authority: Identifier,
    pub issuer: Identifier,
    pub key_id: Identifier,
    pub receipt_id: Identifier,
    pub revocation_ref: Identifier,
    pub revision: u64,
    pub state_digest: Digest,
    pub valid_from: UtcTimestamp,
    pub expires_at: UtcTimestamp,
    pub revoked: bool,
}
