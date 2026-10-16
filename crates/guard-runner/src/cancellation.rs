use tokio::sync::watch;
/// 单向取消信号，不允许复位成新运行。无 Java 对应类型。
#[derive(Clone, Debug)]
pub struct Cancellation {
    sender: watch::Sender<bool>,
}
impl Default for Cancellation {
    fn default() -> Self {
        Self::new()
    }
}
impl Cancellation {
    /// 创建未取消信号，返回可跨任务复制的句柄。
    pub fn new() -> Self {
        Self {
            sender: watch::channel(false).0,
        }
    }
    /// 永久取消当前信号；重复调用幂等，不执行额外动作。
    pub fn cancel(&self) {
        self.sender.send_replace(true);
    }
    /// 返回当前取消状态，不把查询当作受信批准。
    pub fn is_cancelled(&self) -> bool {
        *self.sender.borrow()
    }
    pub(crate) async fn cancelled(&self) {
        let mut receiver = self.sender.subscribe();
        loop {
            if *receiver.borrow_and_update() {
                return;
            }
            if receiver.changed().await.is_err() {
                return;
            }
        }
    }
}
