use crate::{LayerLevel, SourceMaterial};
/// 一个层次的原始闭合协议与来源字节，不包含 trusted 布尔开关。无 Java 对应类型。
#[derive(Clone, Debug)]
pub struct ContractLayer {
    /// 组织、项目或任务层。
    pub level: LayerLevel,
    /// 原始 ContractSnapshot JSON，必须先验 Schema 再验协议语义。
    pub document: Vec<u8>,
    /// 明确提供的来源材料，不隐式读文件或网络。
    pub sources: Vec<SourceMaterial>,
}
