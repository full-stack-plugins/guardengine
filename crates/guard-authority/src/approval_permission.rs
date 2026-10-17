use crate::{ApprovalLayer, ApprovalPurpose};
/// 受保护角色允许的闭合批准用途，不含候选可扩展模式。无Java对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApprovalPermission {
    OrganizationLayer,
    ProjectLayer,
    TaskLayer,
    Task,
    Migration,
}
impl From<&ApprovalPurpose> for ApprovalPermission {
    fn from(value: &ApprovalPurpose) -> Self {
        match value {
            ApprovalPurpose::Layer {
                level: ApprovalLayer::Organization,
            } => Self::OrganizationLayer,
            ApprovalPurpose::Layer {
                level: ApprovalLayer::Project,
            } => Self::ProjectLayer,
            ApprovalPurpose::Layer {
                level: ApprovalLayer::Task,
            } => Self::TaskLayer,
            ApprovalPurpose::Task => Self::Task,
            ApprovalPurpose::Migration { .. } => Self::Migration,
        }
    }
}
