use guard_protocol::SourceRef;
/// 来源原始字节；摘要在本 crate 内重算，不信任调用方声称一致。无 Java 对应类型。
#[derive(Clone, Debug)]
pub struct SourceMaterial {
    /// 来源身份及预期内容摘要。
    pub source: SourceRef,
    /// 实际原始内容，不进行编码/换行归一化。
    pub bytes: Vec<u8>,
}
