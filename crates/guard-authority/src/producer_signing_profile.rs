/// 来源证明专用域，与批准/legacy证据签名严格隔离。
pub const PRODUCER_SIGNING_DOMAIN: &[u8] = b"guard.producer-attestation/v1\n";
/// 原始闭合wire仅删除proof.signature，签入全部身份/算法/key/provenance字段。
pub fn producer_signing_bytes(raw: &[u8]) -> Result<Vec<u8>, String> {
    let doc = parse(raw)?;
    if doc.schema_version != "guard.producer-attestation/v1"
        || doc.provenance.schema_version != "guard.dispatch-provenance/v1"
    {
        return Err("unknown producer source version".into());
    }
    let mut value: serde_json::Value =
        serde_json::from_slice(&guard_protocol::canonicalize(raw)?).map_err(|e| e.to_string())?;
    value["proof"]
        .as_object_mut()
        .ok_or("proof required")?
        .remove("signature");
    let mut bytes = PRODUCER_SIGNING_DOMAIN.to_vec();
    bytes.extend(guard_protocol::canonicalize(
        &serde_json::to_vec(&value).map_err(|e| e.to_string())?,
    )?);
    Ok(bytes)
}
pub(crate) fn parse(raw: &[u8]) -> Result<crate::ProducerAttestation, String> {
    if raw.len() > 1024 * 1024 {
        return Err("producer source exceeds 1MiB".into());
    }
    serde_json::from_slice(&guard_protocol::canonicalize(raw)?).map_err(|e| e.to_string())
}
