use crate::RunTicket;
/// 只有Started允许调度新进程；Existing为同请求幂等观察。无Java对应类型。
#[derive(Clone, Debug)]
pub enum StartOutcome {
    Started(RunTicket),
    Existing(RunTicket),
}
impl StartOutcome {
    /// 提取运行票；调用者仍须保留Started/Existing差异以避免重复启动。
    pub fn into_ticket(self) -> RunTicket {
        match self {
            Self::Started(t) | Self::Existing(t) => t,
        }
    }
}
