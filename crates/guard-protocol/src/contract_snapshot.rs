use crate::{
    ContractObligation, Digest, Extensions, Identifier, RuleDefinition, SchemaVersion, SourceRef,
};
use serde::{Deserialize, Serialize};
/// 任务绑定的不可变批准契约声明。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContractSnapshot {
    /// 冻结协议字段 `schemaVersion`；语义见本 crate 协议表。
    pub schema_version: SchemaVersion,
    /// 冻结协议字段 `contractId`；语义见本 crate 协议表。
    pub contract_id: Identifier,
    /// 冻结协议字段 `version`；语义见本 crate 协议表。
    pub version: Identifier,
    /// 冻结协议字段 `digest`；语义见本 crate 协议表。
    pub digest: Digest,
    /// 冻结协议字段 `sourceRefs`；语义见本 crate 协议表。
    pub source_refs: Vec<SourceRef>,
    /// 冻结协议字段 `rules`；语义见本 crate 协议表。
    pub rules: Vec<RuleDefinition>,
    /// 冻结协议字段 `obligations`；语义见本 crate 协议表。
    pub obligations: Vec<ContractObligation>,
    /// 冻结协议字段 `approvalRef`；语义见本 crate 协议表。
    pub approval_ref: Identifier,
    /// 冻结协议字段 `extensions`；语义见本 crate 协议表。
    pub extensions: Extensions,
}
