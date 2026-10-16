use crate::{Digest, GitOid, ObjectFormat};
use serde::{Deserialize, Serialize};
/// 制品声明的 Git 来源，不允许以工作区/目标 ref 代替固定 commit/tree。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReleaseSourceIdentity {
    /// 来源 Git 对象算法；与 commit/tree 完整长度交叉核验。
    pub object_format: ObjectFormat,
    /// 不可变来源提交 OID，不能接受分支名或自由 ref 表达式。
    pub commit: GitOid,
    /// 来源实际 tree OID；存在性与 commit/tree 关系由后续采集核实。
    pub tree: GitOid,
    /// 来源源码输入清单摘要，独立于制品原始字节摘要。
    pub input_manifest_digest: Digest,
}
