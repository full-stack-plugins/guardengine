use crate::{
    Coverage, DependencyEdge, Digest, Extensions, FactType, Identifier, IndexIdentity,
    ProducerIdentity, ResolutionKind, SchemaVersion, SourceSpan, SymbolReference,
};
use serde::{Deserialize, Serialize};
/// 仅供解释和定位的图谱事实，不能直接签发放行。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FactEnvelope {
    /// 冻结协议字段 `schemaVersion`；语义见本 crate 协议表。
    pub schema_version: SchemaVersion,
    /// 冻结协议字段 `factType`；语义见本 crate 协议表。
    pub fact_type: FactType,
    /// 冻结协议字段 `symbol`；语义见本 crate 协议表。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<SymbolReference>,
    /// 冻结协议字段 `edge`；语义见本 crate 协议表。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge: Option<DependencyEdge>,
    /// 冻结协议字段 `sourceSpan`；语义见本 crate 协议表。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_span: Option<SourceSpan>,
    /// 冻结协议字段 `sourceDigest`；语义见本 crate 协议表。
    pub source_digest: Digest,
    /// 冻结协议字段 `producerIdentity`；语义见本 crate 协议表。
    pub producer_identity: ProducerIdentity,
    /// 冻结协议字段 `indexIdentity`；语义见本 crate 协议表。
    pub index_identity: IndexIdentity,
    /// 冻结协议字段 `resolutionKind`；语义见本 crate 协议表。
    pub resolution_kind: ResolutionKind,
    /// 冻结协议字段 `coverage`；语义见本 crate 协议表。
    pub coverage: Coverage,
    /// 冻结协议字段 `limitations`；语义见本 crate 协议表。
    pub limitations: Vec<Identifier>,
    /// 冻结协议字段 `extensions`；语义见本 crate 协议表。
    pub extensions: Extensions,
}
