use crate::{Digest, Identifier, IngestedDocument, LegacyObservation, ProtocolDocument};
use sha2::{Digest as Hasher, Sha256};
/// 从不受信原始 JSON 解析 v1 文档；拒绝重复键、未知字段/版本和不一致声明。
/// 返回值固定 UNVERIFIED，不核验签名、对象存在性、撤销和实际操作授权。
pub fn decode(bytes: &[u8]) -> Result<IngestedDocument, String> {
    let raw = crate::strict_json::parse(bytes)?;
    // 协议身份不能依赖超出安全整数范围的整数字面值；大数身份使用字符串。
    // 这属于协议输入约束，不缩减 canonicalize 的 RFC8785 数值支持范围。
    crate::validation::safe_integer_literals(&raw)?;
    let canonical = serde_json_canonicalizer::to_vec(&raw).map_err(|e| e.to_string())?;
    let doc: ProtocolDocument = serde_json::from_slice(&canonical).map_err(|e| e.to_string())?;
    crate::validation::validate(&doc)?;
    Ok(IngestedDocument {
        document: doc,
        original: bytes.to_vec(),
    })
}
/// 对合法协议声明产生规范字节；类型合法不代表其来源或批准已核实。
pub fn encode(doc: &ProtocolDocument) -> Result<Vec<u8>, String> {
    crate::validation::validate(doc)?;
    let bytes = serde_json::to_vec(doc).map_err(|e| e.to_string())?;
    // 再经严格入口检查公共字段被修改后的不一致和扩展数字限制。
    decode(&bytes)?;
    crate::canonicalize(&bytes)
}
/// 明确适配历史 0.x 或旧 GuardEngine alpha 原件；只保留审计线索，不推导工程通过。
/// adapter_version 必须显式给出生产者/版本，返回原始字节及固定未验证状态。
pub fn project_legacy(bytes: &[u8], adapter_version: &str) -> Result<LegacyObservation, String> {
    let raw = crate::strict_json::parse(bytes)?;
    let adapter = Identifier::new(adapter_version)?;
    let (owner, version) = adapter_version
        .split_once('/')
        .ok_or("adapterVersion requires producer/version")?;
    if ![
        "codeguard",
        "codegraph",
        "codereview",
        "flowguard",
        "gitflow",
        "guardengine",
    ]
    .contains(&owner)
        || version.is_empty()
        || version == "latest"
    {
        return Err("unsupported or unversioned legacy adapter".into());
    }
    let known = raw
        .get("schema_version")
        .or_else(|| raw.get("schemaVersion"))
        .and_then(|v| v.as_str())
        .is_some_and(|s| {
            let parts = s.split('.').collect::<Vec<_>>();
            parts.len() == 3
                && parts[0] == "0"
                && parts
                    .iter()
                    .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
        })
        || raw
            .get("apiVersion")
            .and_then(|v| v.as_str())
            .is_some_and(|v| matches!(v, "guard.partme.ai/v1alpha1" | "guard.partme.ai/v1alpha2"));
    if !known {
        return Err("unknown legacy version; protocol incomplete".into());
    }
    let digest = Digest::new(format!("sha256:{:x}", Sha256::digest(bytes)))?;
    Ok(LegacyObservation {
        original: bytes.to_vec(),
        adapter,
        digest,
    })
}
