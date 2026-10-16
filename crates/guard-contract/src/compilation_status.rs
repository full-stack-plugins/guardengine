use serde::Serialize;
/// 编译准备状态，不是工程门禁或执行授权。无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompilationStatus {
    /// 不满足批准、来源、规则或能力条件。
    Blocked,
    /// 可供后续K03构造冻结计划，但本包不生成执行计划。
    ReadyForPlanning,
}
