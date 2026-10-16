use crate::{CaptureBinding, InputEntry, Selection};
use guard_protocol::{Digest, SubjectSnapshot};
use serde::{Deserialize, Serialize};
/// 候选快照、显式绑定和输入清单；不是通过报告或受信身份。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapturedSubject {
    /// v1 候选协议字段。
    pub snapshot: SubjectSnapshot,
    /// 调用端的全部显式身份绑定。
    pub binding: CaptureBinding,
    /// 精确采集请求，防止跨候选种类复用。
    pub selection: Selection,
    /// 按原始路径字节排序的实际输入清单。
    pub manifest: Vec<InputEntry>,
    /// 包含类型、请求、协议快照及全部绑定的规范摘要。
    pub identity_digest: Digest,
}
