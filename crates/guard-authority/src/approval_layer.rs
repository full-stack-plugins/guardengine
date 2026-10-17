use guard_contract::LayerLevel;
use serde::{Deserialize, Serialize};
/// 闭合层次wire；现有LayerLevel只有Serialize，避免改动其crate。无Java对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalLayer {
    Organization,
    Project,
    Task,
}
impl From<LayerLevel> for ApprovalLayer {
    fn from(value: LayerLevel) -> Self {
        match value {
            LayerLevel::Organization => Self::Organization,
            LayerLevel::Project => Self::Project,
            LayerLevel::Task => Self::Task,
        }
    }
}
