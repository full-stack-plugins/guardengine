use guard_protocol::Digest;
use serde::{Deserialize, Serialize};
/// 策略包中相对文件与原始字节摘要；无Java对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PolicyFile {
    pub path: String,
    pub digest: Digest,
}
