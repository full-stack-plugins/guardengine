use guard_protocol::Identifier;
use serde::Serialize;
/// 本次任务及已有授权范围，不是候选自报的受信批准。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompileContext {
    /// 目标仓库身份，不能由本地目录或remote URL猜测。
    pub repository_id: Identifier,
    /// 当前任务身份。
    pub task_id: Identifier,
    /// 已有授权引用，须由独立端口核验。
    pub authorization_refs: Vec<Identifier>,
}
