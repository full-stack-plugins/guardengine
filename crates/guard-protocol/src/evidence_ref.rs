use crate::{Classification, Digest, Identifier, Retention};
use serde::{Deserialize, Serialize};
/// 内容寻址原始报告或摘要引用；声明不能代替真实文件核验。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceRef {
    /// 冻结协议字段 `digest`；语义见本 crate 协议表。
    pub digest: Digest,
    /// 冻结协议字段 `mediaType`；语义见本 crate 协议表。
    pub media_type: Identifier,
    /// 冻结协议字段 `size`；语义见本 crate 协议表。
    pub size: u64,
    /// 冻结协议字段 `classification`；语义见本 crate 协议表。
    pub classification: Classification,
    /// 冻结协议字段 `retention`；语义见本 crate 协议表。
    pub retention: Retention,
}
