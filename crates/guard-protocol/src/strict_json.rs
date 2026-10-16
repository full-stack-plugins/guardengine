use serde::de::{Error, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Number, Value};
use std::fmt;
/// 严格 JSON 读取包装，确保所有嵌套层拒绝重复键和非有限数字。无 Java 对应类型。
pub(crate) struct StrictJson(pub Value);
impl<'de> Deserialize<'de> for StrictJson {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Read;
        impl<'de> Visitor<'de> for Read {
            type Value = StrictJson;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("bounded JSON with unique keys and finite IEEE754 numbers")
            }
            fn visit_bool<E: Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(StrictJson(Value::Bool(v)))
            }
            fn visit_unit<E: Error>(self) -> Result<Self::Value, E> {
                Ok(StrictJson(Value::Null))
            }
            fn visit_str<E: Error>(self, v: &str) -> Result<Self::Value, E> {
                self.visit_string(v.to_owned())
            }
            fn visit_string<E: Error>(self, v: String) -> Result<Self::Value, E> {
                if v.len() > 65536 {
                    return Err(E::custom("JSON string limit"));
                }
                Ok(StrictJson(Value::String(v)))
            }
            fn visit_i64<E: Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(StrictJson(Value::Number(Number::from(v))))
            }
            fn visit_u64<E: Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(StrictJson(Value::Number(Number::from(v))))
            }
            fn visit_f64<E: Error>(self, value: f64) -> Result<Self::Value, E> {
                Number::from_f64(value)
                    .map(|n| StrictJson(Value::Number(n)))
                    .ok_or_else(|| E::custom("non-finite IEEE754 number"))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut rows = Vec::new();
                while let Some(value) = seq.next_element::<StrictJson>()? {
                    if rows.len() >= 8192 {
                        return Err(A::Error::custom("JSON array limit"));
                    }
                    rows.push(value.0);
                }
                Ok(StrictJson(Value::Array(rows)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut fields = Map::new();
                while let Some(key) = map.next_key::<String>()? {
                    if key.len() > 4096 || fields.len() >= 2048 {
                        return Err(A::Error::custom("JSON object limit"));
                    }
                    if fields.contains_key(&key) {
                        return Err(A::Error::custom("duplicate JSON key"));
                    }
                    fields.insert(key, map.next_value::<StrictJson>()?.0);
                }
                Ok(StrictJson(Value::Object(fields)))
            }
        }
        d.deserialize_any(Read)
    }
}
pub(crate) fn parse(bytes: &[u8]) -> Result<Value, String> {
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("JSON input exceeds 2 MiB".into());
    }
    let value = serde_json::from_slice::<StrictJson>(bytes)
        .map_err(|e| e.to_string())?
        .0;
    fn depth(v: &Value, n: usize) -> Result<(), String> {
        if n > 64 {
            return Err("JSON nesting exceeds 64".into());
        }
        match v {
            Value::Array(a) => {
                for v in a {
                    depth(v, n + 1)?;
                }
            }
            Value::Object(o) => {
                for v in o.values() {
                    depth(v, n + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    depth(&value, 0)?;
    Ok(value)
}
