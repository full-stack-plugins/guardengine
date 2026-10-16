use crate::{Coverage, Digest, Identifier, Verdict};
use serde::{Deserialize, Serialize};
/// 事实判定与范围覆盖独立记录，保留适用性和问题引用。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Assessment {
    /// 冻结协议字段 `verdict`；语义见本 crate 协议表。
    pub verdict: Verdict,
    /// 冻结协议字段 `coverage`；语义见本 crate 协议表。
    pub coverage: Coverage,
    /// 冻结协议字段 `applicabilityEvidenceRefs`；语义见本 crate 协议表。
    pub applicability_evidence_refs: Vec<Digest>,
    /// 冻结协议字段 `findingIds`；语义见本 crate 协议表。
    pub finding_ids: Vec<Identifier>,
}
