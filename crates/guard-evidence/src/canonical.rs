use serde_json::Value;
use sha2::{Digest, Sha256};

pub(crate) fn canonical(value: &Value) -> Result<Vec<u8>, String> {
    fn encode(value: &Value, output: &mut Vec<u8>) -> Result<(), String> {
        match value {
            Value::Object(object) => {
                output.push(b'{');
                let mut keys: Vec<_> = object.keys().collect();
                keys.sort();
                for (index, key) in keys.into_iter().enumerate() {
                    if index > 0 {
                        output.push(b',');
                    }
                    output.extend(serde_json::to_vec(key).map_err(|e| e.to_string())?);
                    output.push(b':');
                    encode(&object[key], output)?;
                }
                output.push(b'}');
            }
            Value::Array(values) => {
                output.push(b'[');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        output.push(b',');
                    }
                    encode(value, output)?;
                }
                output.push(b']');
            }
            _ => output.extend(serde_json::to_vec(value).map_err(|e| e.to_string())?),
        }
        Ok(())
    }
    let mut output = Vec::new();
    encode(value, &mut output)?;
    Ok(output)
}
/// 对递归键排序的 JSON 计算 sha256 摘要，数组保留顺序；无 Java 对应方法。
pub fn digest_json(value: &Value) -> Result<String, String> {
    Ok(format!("sha256:{:x}", Sha256::digest(canonical(value)?)))
}
