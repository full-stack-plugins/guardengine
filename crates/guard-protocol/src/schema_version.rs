use serde::{Deserialize, Deserializer, Serialize};
/// 已冻结的协议版本；未知版本不能降级为允许。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct SchemaVersion(String);
impl SchemaVersion {
    /// 校验并创建值；参数为原始文本，返回合法值或具体错误。
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        if value == "1.0.0" {
            Ok(Self(value))
        } else {
            Err("invalid SchemaVersion".into())
        }
    }
    /// 返回原始文本，不做大小写折叠或 Unicode 归一化。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl<'de> Deserialize<'de> for SchemaVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}
