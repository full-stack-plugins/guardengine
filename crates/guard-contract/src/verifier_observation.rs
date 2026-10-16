use crate::VerifierPin;
/// 只读能力观测不代表实际执行成功。无 Java 对应类型。
#[derive(Clone, Debug)]
pub enum VerifierObservation {
    /// 外部观测的实际身份，仍须与固定pin精确比对。
    Available(VerifierPin),
    /// 既有工具不存在，不自动安装。
    Missing(String),
    /// 平台或构建不支持，不伪造执行。
    Unsupported(String),
    /// 无法可靠观测，保守阻塞。
    Unknown(String),
}
