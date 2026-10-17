use guard_protocol::{Decision, Identifier};
use serde::{Deserialize, Serialize};
use serde_json::Value;
/// OPA返回的闭合策略结论，不是GateDecision、授权或受信生产者证明。无Java对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PolicyDecision {
    pub schema_version: u32,
    pub decision: Decision,
    pub reasons: Vec<Identifier>,
    pub required_actions: Vec<Identifier>,
    pub missing_obligations: Vec<Identifier>,
    pub active_violations: Vec<Value>,
}
impl PolicyDecision {
    /// 仅接受单一固定data查询的唯一完整JSON值；undefined/errors/未知字段均拒绝。
    pub fn from_opa_json(bytes: &[u8], query: &str) -> Result<Self, String> {
        if bytes.len() > 1024 * 1024 {
            return Err("OPA JSON exceeds 1MiB".into());
        }
        let raw = guard_protocol::canonicalize(bytes)?;
        let value: Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
        let top = value.as_object().ok_or("OPA envelope object required")?;
        if top.len() != 1 || !top.contains_key("result") {
            return Err("OPA undefined/errors/unexpected envelope".into());
        }
        let result = value["result"]
            .as_array()
            .filter(|a| a.len() == 1)
            .ok_or("OPA unique defined result required")?;
        let row = result[0].as_object().ok_or("OPA result object required")?;
        if row
            .keys()
            .any(|k| !["expressions", "bindings"].contains(&k.as_str()))
            || row
                .get("bindings")
                .is_some_and(|v| v != &serde_json::json!({}))
        {
            return Err("OPA bindings/unexpected result fields".into());
        }
        let expressions = row
            .get("expressions")
            .ok_or("OPA expressions required")?
            .as_array()
            .filter(|a| a.len() == 1)
            .ok_or("OPA unique expression required")?;
        let expression = expressions[0]
            .as_object()
            .ok_or("OPA expression object required")?;
        if expression.len() != 3
            || expression.get("text") != Some(&Value::String(query.to_string()))
            || expression.get("location") != Some(&serde_json::json!({"row":1,"col":1}))
        {
            return Err("OPA expression does not match fixed query".into());
        }
        let out: Self =
            serde_json::from_value(expression.get("value").ok_or("OPA value required")?.clone())
                .map_err(|e| e.to_string())?;
        if out.schema_version != 1
            || [
                out.reasons.len(),
                out.required_actions.len(),
                out.missing_obligations.len(),
                out.active_violations.len(),
            ]
            .iter()
            .any(|v| *v > 4096)
            || out.active_violations.iter().any(|v| !v.is_object())
        {
            return Err("unsupported/bounded OPA policy decision required".into());
        }
        Ok(out)
    }
}
