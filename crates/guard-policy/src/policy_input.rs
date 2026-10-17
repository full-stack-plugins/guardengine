use crate::PolicyBinding;
use guard_protocol::Digest;
use serde::{Deserialize, Serialize};
use serde_json::Value;
/// 闭合控制信封和领域opaque facts；不将facts中trusted/approval文字变成权威。无Java对应类型。
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PolicyInput {
    schema_version: String,
    binding: PolicyBinding,
    facts: Value,
}
impl PolicyInput {
    /// 读取显式本地输入，只有BoundPolicyEvaluator能确认与真实注册计划相同。
    pub fn from_json(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > 1024 * 1024 {
            return Err("policy input exceeds 1MiB".into());
        }
        let raw = guard_protocol::canonicalize(bytes)?;
        let input: Self = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
        if input.schema_version != "guard.policy-input/v1" || !input.facts.is_object() {
            return Err("unsupported policy input envelope".into());
        }
        Ok(input)
    }
    /// 用显式身份包裹领域事实，不证明身份或事实来源。
    pub fn from_facts(binding: PolicyBinding, facts: &[u8]) -> Result<Self, String> {
        let value: Value = serde_json::from_slice(&guard_protocol::canonicalize(facts)?)
            .map_err(|e| e.to_string())?;
        Self::from_json(
            &serde_json::to_vec(&Self {
                schema_version: "guard.policy-input/v1".into(),
                binding,
                facts: value,
            })
            .map_err(|e| e.to_string())?,
        )
    }
    /// 返回固定信封绑定，不接受事实覆盖其字段。
    pub fn binding(&self) -> &PolicyBinding {
        &self.binding
    }
    /// 返回canonical输入摘要，绑定实际OPA读取的同一组字节。
    pub fn digest(&self) -> Result<Digest, String> {
        crate::policy_package::json_hash(self)
    }
    pub(crate) fn bytes(&self) -> Result<Vec<u8>, String> {
        guard_protocol::canonicalize(&serde_json::to_vec(self).map_err(|e| e.to_string())?)
    }
}
