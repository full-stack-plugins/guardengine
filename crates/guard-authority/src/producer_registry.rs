use crate::ProducerRegistration;
use guard_protocol::Identifier;
/// 由受保护宿主注入的生产者注册来源，不接受候选自选信任锚。无Java对应类型。
pub trait ProducerRegistry: Send + Sync {
    /// 按生产者及消费目标读取当前独立配置；缺失/不可用必须拒绝。
    fn current(
        &self,
        producer: &Identifier,
        audience: &Identifier,
        repository: &Identifier,
        task: &Identifier,
    ) -> Result<ProducerRegistration, String>;
}
