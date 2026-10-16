use serde::{Deserialize, Deserializer, Serialize};
/// 发布制品逻辑名称，采用受限 ASCII basename；不是本地路径或存储位置。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ArtifactName(String);
impl ArtifactName {
    /// 校验非路径名称，拒绝目录、控制字符、歧义点段及 Windows 设备名。
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        let base = value.split('.').next().unwrap_or("").to_ascii_uppercase();
        let reserved = ["CON", "PRN", "AUX", "NUL"].contains(&base.as_str())
            || (base.len() == 4
                && (base.starts_with("COM") || base.starts_with("LPT"))
                && (b'1'..=b'9').contains(&base.as_bytes()[3]));
        if value.is_empty()
            || value.len() > 255
            || !value.as_bytes()[0].is_ascii_alphanumeric()
            || value.ends_with('.')
            || value.contains("..")
            || reserved
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        {
            Err("invalid non-path artifact name".into())
        } else {
            Ok(Self(value))
        }
    }
    /// 返回逻辑名称；不能把它当作已批准写入路径。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl<'de> Deserialize<'de> for ArtifactName {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}
