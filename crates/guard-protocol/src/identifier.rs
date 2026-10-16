use serde::{Deserialize, Deserializer, Serialize};
/// 非空有界标识；不赋予仓库、用户或生产者任何信任。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct Identifier(String);
impl Identifier {
    /// 校验并创建值；参数为原始文本，返回合法值或具体错误。
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        if !value.trim().is_empty() && value.len() <= 4096 && !value.chars().any(char::is_control) {
            Ok(Self(value))
        } else {
            Err("invalid Identifier".into())
        }
    }
    /// 返回原始文本，不做大小写折叠或 Unicode 归一化。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl<'de> Deserialize<'de> for Identifier {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}
