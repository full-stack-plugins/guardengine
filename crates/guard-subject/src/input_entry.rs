use guard_protocol::{Digest, GitOid, GitPath};
use serde::{Deserialize, Serialize};
/// 原始路径、文件模式、Git blob 身份与原始内容摘要。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputEntry {
    /// 无损原始相对路径字节。
    pub path: GitPath,
    /// Git 100644/100755 模式，其他模式显式拒绝。
    pub mode: String,
    /// Git 候选文件的实际 blob OID；工作树原始文件不伪造 Git blob 身份。
    pub object_id: Option<GitOid>,
    /// 原始文件或 blob 字节 sha256，不做换行或编码规范化。
    pub content_digest: Digest,
}
