use crate::{ApprovalPurpose, ApprovedScope};
use guard_protocol::{Action, Identifier};
use serde::{Deserialize, Serialize};
/// 由provenanceDigest绑定的闭合授权材料；仅解析不授信。无Java对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovalMaterial {
    pub schema_version: String,
    pub audience: Identifier,
    pub requested_by: Identifier,
    pub action: Action,
    pub purpose: ApprovalPurpose,
    pub scope: ApprovedScope,
}
impl ApprovalMaterial {
    /// 严格读取有界材料，重复key及未知字段拒绝；返回未核验的数据。
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > 1024 * 1024 {
            return Err("approval material exceeds 1MiB".into());
        }
        let canonical = guard_protocol::canonicalize(bytes)?;
        let value: Self = serde_json::from_slice(&canonical).map_err(|e| e.to_string())?;
        if value.schema_version != "guard.approval-material/v1"
            || !value.scope.actions.contains(&value.action)
        {
            return Err("unknown approval material or action outside scope".into());
        }
        value.scope.digest()?;
        Ok(value)
    }
}
