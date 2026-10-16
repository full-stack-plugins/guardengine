use crate::VerifierKind;
use guard_protocol::{Digest, DomainOwner, Identifier};
use serde::Serialize;
/// 编译结果固定的验证器精确身份，不包含可执行argv。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifierPin {
    /// 固定验证器ID。
    pub verifier_id: Identifier,
    /// 固定版本。
    pub version: Identifier,
    /// 固定制品摘要。
    pub artifact_digest: Digest,
    /// 固定配置摘要。
    pub config_digest: Digest,
    /// 固定适配器版本。
    pub adapter_version: Identifier,
    /// 验证器职责。
    pub kind: VerifierKind,
    /// 专业owner归属。
    pub owner: Option<DomainOwner>,
    /// 可产生的固定证据种类，规则不能引用未知证据。
    pub evidence_types: Vec<Identifier>,
    /// 实际参数Schema原件摘要。
    pub parameter_schema_digest: Digest,
}
