use crate::{SchemaValidator, VerifierPin};
/// 注册后的固定身份及真实参数求值器；只在受信registry内保存。无 Java 对应类型。
pub(crate) struct RegisteredVerifier {
    pub pin: VerifierPin,
    pub parameters: SchemaValidator,
}
