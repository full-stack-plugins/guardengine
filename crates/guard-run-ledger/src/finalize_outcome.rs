/// 终结请求对当前头的影响；AuditOnly永远不恢复当前资格。无Java对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinalizeOutcome {
    Applied,
    Unchanged,
    AuditOnly,
}
