/// 只读领域上下文提供者；输入显式目标与可信适配器选项，返回结构化上下文或错误。
pub type ContextProvider = fn(&crate::Options) -> Result<serde_json::Value, String>;
