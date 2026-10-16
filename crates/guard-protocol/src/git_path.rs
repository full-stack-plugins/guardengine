use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Deserializer, Serialize};
/// Git 路径原始字节的显式编码，禁止解码碰撞和目录穿越。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitPath {
    encoding: String,
    bytes: String,
}
impl GitPath {
    /// 从原始相对路径创建规范 base64 表示；返回拒绝非法路径的结果。
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.is_empty()
            || bytes.len() > 4096
            || bytes.contains(&0)
            || bytes.starts_with(b"/")
            || bytes
                .split(|b| *b == b'/')
                .any(|part| part.is_empty() || part == b"." || part == b"..")
        {
            return Err("invalid Git path bytes".into());
        }
        Ok(Self {
            encoding: "base64".into(),
            bytes: STANDARD.encode(bytes),
        })
    }
    /// 解码为原始字节；不归一化大小写、Unicode 或非法 UTF-8。
    pub fn to_bytes(&self) -> Vec<u8> {
        STANDARD.decode(&self.bytes).expect("validated Git path")
    }
}
impl<'de> Deserialize<'de> for GitPath {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            encoding: String,
            bytes: String,
        }
        let wire = Wire::deserialize(d)?;
        if wire.encoding != "base64" {
            return Err(serde::de::Error::custom("Git path encoding must be base64"));
        }
        let decoded = STANDARD
            .decode(&wire.bytes)
            .map_err(serde::de::Error::custom)?;
        let value = Self::from_bytes(&decoded).map_err(serde::de::Error::custom)?;
        if value.bytes != wire.bytes {
            return Err(serde::de::Error::custom("noncanonical base64"));
        }
        Ok(value)
    }
}
