use crate::VerifierKind;
use guard_protocol::{Digest, DomainOwner, Identifier};
/// 受信配置注册的固定验证器描述，不从候选规则动态安装或解析 latest。无 Java 对应类型。
#[derive(Clone, Debug)]
pub struct VerifierDescriptor {
    /// 固定ID。
    pub verifier_id: Identifier,
    /// 固定版本字符串，不接受范围或latest。
    pub version: Identifier,
    /// 必须核验的可执行制品摘要。
    pub artifact_digest: Digest,
    /// 固定配置摘要。
    pub config_digest: Digest,
    /// 固定适配器版本。
    pub adapter_version: Identifier,
    /// 职责种类。
    pub kind: VerifierKind,
    /// 专业验证器必须声明专业owner，OPA不声明。
    pub owner: Option<DomainOwner>,
    /// 可产生的固定证据种类，规则不能引用未知证据。
    pub evidence_types: Vec<Identifier>,
    /// 真实 Draft2020-12 参数 Schema 原始字节。
    pub parameter_schema: Vec<u8>,
}
