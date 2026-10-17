use guard_protocol::ExecutionStatus;
use guard_run_ledger::FinalizeOutcome;

/// 映射机械执行的CLI退出码；后置复核失败或仅审计记录不称执行闭环完成。无Java对应对象。
pub(crate) fn code(
    status: ExecutionStatus,
    exit: Option<i32>,
    drifted: bool,
    truncated: bool,
    recheck_error: Option<&str>,
    finalization: Option<FinalizeOutcome>,
    cancelled: bool,
) -> u8 {
    if cancelled {
        130
    } else if status == ExecutionStatus::Completed
        && exit == Some(0)
        && !drifted
        && !truncated
        && recheck_error.is_none()
        && matches!(finalization, Some(FinalizeOutcome::Applied))
    {
        0
    } else {
        3
    }
}

#[cfg(test)]
mod tests {
    use super::code;
    use guard_protocol::ExecutionStatus;
    use guard_run_ledger::FinalizeOutcome;
    #[test]
    fn completion_requires_postcheck_and_current_applied_finalization() {
        assert_eq!(
            code(
                ExecutionStatus::Completed,
                Some(0),
                false,
                false,
                None,
                Some(FinalizeOutcome::Applied),
                false
            ),
            0
        );
        assert_eq!(
            code(
                ExecutionStatus::Completed,
                Some(0),
                false,
                false,
                Some("current check timed out"),
                Some(FinalizeOutcome::Applied),
                false
            ),
            3
        );
        assert_eq!(
            code(
                ExecutionStatus::Completed,
                Some(0),
                false,
                false,
                None,
                Some(FinalizeOutcome::AuditOnly),
                false
            ),
            3
        );
        assert_eq!(
            code(
                ExecutionStatus::Completed,
                Some(0),
                false,
                false,
                None,
                None,
                false
            ),
            3
        );
    }
}
