use sha2::{Digest as Hasher, Sha256};
/// 按 RFC 8785/JCS 规范化原始 JSON，包括完整有限 IEEE754 数值、负零、指数和 UTF-16 键序。
/// 首先逐层拒绝重复键、非法 Unicode 与非有限值，再使用固定版本 ECMAScript 序列化器。
pub fn canonicalize(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let value = crate::strict_json::parse(bytes)?;
    serde_json_canonicalizer::to_vec(&value).map_err(|e| e.to_string())
}
/// 对 RFC 8785/JCS 规范字节计算 sha256；参数为 JSON 原始字节。
pub fn digest_json(bytes: &[u8]) -> Result<crate::Digest, String> {
    crate::Digest::new(format!("sha256:{:x}", Sha256::digest(canonicalize(bytes)?)))
}
