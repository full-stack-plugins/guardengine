use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
/// 命名空间扩展只能携带元数据，不能覆盖核心规则和授权。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize)]
#[serde(transparent)]
pub struct Extensions(BTreeMap<String, Value>);
impl Extensions {
    /// 读取扩展元数据；消费端不能把它作为核心决定字段。
    pub fn values(&self) -> &BTreeMap<String, Value> {
        &self.0
    }
}
impl<'de> Deserialize<'de> for Extensions {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let values = BTreeMap::<String, Value>::deserialize(d)?;
        for key in values.keys() {
            let Some((namespace, name)) = key.split_once(':') else {
                return Err(serde::de::Error::custom(
                    "extension requires namespace:name",
                ));
            };
            if namespace.is_empty()
                || name.is_empty()
                || key.len() > 256
                || !namespace
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
                || !namespace.as_bytes()[0].is_ascii_lowercase()
                || !name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
            {
                return Err(serde::de::Error::custom("invalid extension namespace"));
            }
        }
        Ok(Self(values))
    }
}
