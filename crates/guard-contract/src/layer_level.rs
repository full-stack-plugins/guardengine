use serde::Serialize;
/// 只读组合的固定层次，组织层永远先于项目和任务层。无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LayerLevel {
    /// 组织基线规则全部锁定。
    Organization,
    /// 项目补充约束。
    Project,
    /// 任务补充约束。
    Task,
}
