use guard_protocol::{ContractSnapshot, Digest, ProtocolDocument};
/// 计算快照内容摘要：完整v1文档仅排除自引用digest字段，其余版本/来源/批准引用均进入JCS。
/// 这是候选身份计算，不核实批准，不生成受信计划。
pub fn snapshot_digest(snapshot: &ContractSnapshot) -> Result<Digest, String> {
    let mut value = serde_json::to_value(ProtocolDocument::ContractSnapshot(snapshot.clone()))
        .map_err(|e| e.to_string())?;
    value
        .as_object_mut()
        .ok_or("snapshot object required")?
        .remove("digest");
    guard_protocol::digest_json(&serde_json::to_vec(&value).map_err(|e| e.to_string())?)
}
