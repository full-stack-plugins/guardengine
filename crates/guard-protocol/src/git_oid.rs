use serde::{Deserialize, Deserializer, Serialize};
/// 无损 Git 对象 ID；实际对象格式由所属快照交叉校验。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct GitOid(String);
impl GitOid {
    /// 校验并创建值；参数为原始文本，返回合法值或具体错误。
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        if matches!(value.len(), 40 | 64)
            && value
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            Ok(Self(value))
        } else {
            Err("invalid GitOid".into())
        }
    }
    /// 返回原始文本，不做大小写折叠或 Unicode 归一化。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl<'de> Deserialize<'de> for GitOid {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}
