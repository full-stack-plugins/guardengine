use crate::{VerifierObservation, VerifierPin};
/// 注入的只读本机能力观测端口；实现方不得安装、联网、建索引或启动检查。
pub trait CapabilityProbe {
    /// 返回现有制品/配置的已观测身份或缺失原因，不执行验证器。
    fn observe(&self, pin: &VerifierPin) -> VerifierObservation;
}
