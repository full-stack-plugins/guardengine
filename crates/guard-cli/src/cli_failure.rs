/// CLI边界的明确失败类别；不按工具退出数字或易变错误正文猜质量。无Java对应类型。
#[derive(Debug)]
pub(crate) enum CliFailure {
    Usage(String),
    Rejected(String),
    Incomplete(String),
    Internal(String),
}
impl CliFailure {
    /// 返回错误种类的固定退出码，不猜测错误文本。
    pub(crate) fn code(&self) -> u8 {
        match self {
            Self::Usage(_) => 2,
            Self::Rejected(_) => 1,
            Self::Incomplete(_) => 3,
            Self::Internal(_) => 4,
        }
    }
    /// 返回对应故障说明，供结构化诊断使用。
    pub(crate) fn message(&self) -> &str {
        match self {
            Self::Usage(v) | Self::Rejected(v) | Self::Incomplete(v) | Self::Internal(v) => v,
        }
    }
}
impl From<String> for CliFailure {
    fn from(value: String) -> Self {
        Self::Incomplete(value)
    }
}
impl From<&str> for CliFailure {
    fn from(value: &str) -> Self {
        Self::Incomplete(value.into())
    }
}
