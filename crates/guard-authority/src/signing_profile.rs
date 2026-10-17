use guard_protocol::{ApprovalReceipt, ProtocolDocument};
/// 固定K09签名域字节；与legacy证据签名严格区分。
pub const SIGNING_DOMAIN: &[u8] = b"guard.approval-receipt/v1\n";
/// 为typed producer构造真实签名材料；不签名、不证明授权。
pub fn signing_bytes(receipt: &ApprovalReceipt) -> Result<Vec<u8>, String> {
    wire_signing_bytes(
        &serde_json::to_vec(&ProtocolDocument::ApprovalReceipt(receipt.clone()))
            .map_err(|e| e.to_string())?,
    )
}
/// 从原始wire构造签名材料，仅剔除proof.signature，绝不typed重序列化丢弃null字段。
pub fn wire_signing_bytes(raw: &[u8]) -> Result<Vec<u8>, String> {
    let decoded = guard_protocol::decode(raw)?;
    if !matches!(decoded.document(), ProtocolDocument::ApprovalReceipt(_)) {
        return Err("ApprovalReceipt signing profile required".into());
    }
    let mut value: serde_json::Value =
        serde_json::from_slice(&guard_protocol::canonicalize(raw)?).map_err(|e| e.to_string())?;
    value["proof"]
        .as_object_mut()
        .ok_or("proof missing")?
        .remove("signature");
    let mut bytes = SIGNING_DOMAIN.to_vec();
    bytes.extend(guard_protocol::canonicalize(
        &serde_json::to_vec(&value).map_err(|e| e.to_string())?,
    )?);
    Ok(bytes)
}
