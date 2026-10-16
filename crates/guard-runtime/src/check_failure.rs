/// 候选复核失效与准备中止分离，不能把预算耗尽谎称输入漂移。无Java对应类型。
#[derive(Debug)]
pub(crate) enum CheckFailure {
    Rejected(String),
    TimedOut,
    Cancelled,
}
