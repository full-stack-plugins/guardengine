use jsonschema::{Retrieve, Uri};
use serde_json::Value;
/// 永远拒绝外部 Schema 检索；即使调用方合并开启依赖 feature 也不读取网络或文件。
pub(crate) struct DenyExternal;
impl Retrieve for DenyExternal {
    fn retrieve(
        &self,
        _uri: &Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        Err("external Schema resolution is disabled".into())
    }
}
