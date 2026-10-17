use crate::{PolicyArtifactRef, PolicyEvaluation};
use serde::Serialize;
/// 独立metadata原件及投影，不支持从JSON恢复批准或运行资格。无Java对应类型。
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyReceipt {
    pub evaluation: PolicyEvaluation,
    pub artifact: PolicyArtifactRef,
}
