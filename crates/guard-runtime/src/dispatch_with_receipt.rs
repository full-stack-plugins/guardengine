use crate::{DispatchCompletion, DispatchOutcome};
/// 原观察与可选真实派发凭据；观察本身不构成资格。无Java对应类型。
#[derive(Debug)]
pub struct DispatchWithReceipt {
    pub(crate) outcome: DispatchOutcome,
    pub(crate) completion: Option<DispatchCompletion>,
    pub(crate) qualification_error: Option<String>,
}
impl DispatchWithReceipt {
    /// 附属来源凭据采集失败原因；不抹去原观察，也不伪称取得资格。
    pub fn qualification_error(&self) -> Option<&str> {
        self.qualification_error.as_deref()
    }

    /// 消费包装并取出原观察与可选内部凭据，不从观察构造凭据。
    pub fn into_parts(self) -> (DispatchOutcome, Option<DispatchCompletion>) {
        (self.outcome, self.completion)
    }
}
