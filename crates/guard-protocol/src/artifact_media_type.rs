use serde::{Deserialize, Deserializer, Serialize};
/// 受限小写 type/subtype 媒体标识，不接受参数、路径或头注入。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ArtifactMediaType(String);
impl ArtifactMediaType {
    /// 校验两个非空小写 ASCII token；参数、空白和额外 slash 均拒绝。
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        let token = |part: &str| {
            !part.is_empty()
                && part.bytes().all(|b| {
                    b.is_ascii_lowercase()
                        || b.is_ascii_digit()
                        || matches!(
                            b,
                            b'!' | b'#' | b'$' | b'&' | b'^' | b'_' | b'.' | b'+' | b'-'
                        )
                })
        };
        if value.len() > 255
            || !value
                .split_once('/')
                .is_some_and(|(kind, subtype)| token(kind) && token(subtype))
        {
            Err("invalid artifact media type".into())
        } else {
            Ok(Self(value))
        }
    }
    /// 返回规范小写标识，不产生内容类型真实性证明。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl<'de> Deserialize<'de> for ArtifactMediaType {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}
