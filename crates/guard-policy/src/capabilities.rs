use serde_json::Value;
use std::collections::BTreeSet;
pub(crate) fn validate(bytes: &[u8]) -> Result<(), String> {
    let raw = guard_protocol::canonicalize(bytes)?;
    let value: Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    let object = value.as_object().ok_or("capabilities object required")?;
    if object.keys().any(|key| {
        ![
            "builtins",
            "allow_net",
            "future_keywords",
            "wasm_abi_versions",
            "features",
        ]
        .contains(&key.as_str())
    }) || value["allow_net"] != serde_json::json!([])
    {
        return Err("closed capabilities with explicit empty allow_net required".into());
    }
    let allowed = [
        "internal.member_2",
        "internal.member_3",
        "and",
        "array.concat",
        "array.slice",
        "concat",
        "count",
        "crypto.sha256",
        "div",
        "eq",
        "equal",
        "floor",
        "gt",
        "gte",
        "is_array",
        "is_boolean",
        "is_null",
        "is_number",
        "is_object",
        "is_set",
        "is_string",
        "json.marshal",
        "json.unmarshal",
        "lt",
        "lte",
        "minus",
        "mul",
        "neq",
        "numbers.range",
        "object.get",
        "object.keys",
        "or",
        "plus",
        "regex.match",
        "rem",
        "set_diff",
        "sort",
        "sprintf",
        "startswith",
    ];
    let builtins = value["builtins"]
        .as_array()
        .ok_or("capabilities builtins required")?;
    if builtins.len() > 128 {
        return Err("capabilities builtin count budget".into());
    }
    let mut names = BTreeSet::new();
    for builtin in builtins {
        let name = builtin["name"].as_str().ok_or("builtin name required")?;
        if !allowed.contains(&name) || !names.insert(name) {
            return Err(format!("unsupported or duplicate non-pure builtin: {name}"));
        }
    }
    if let Some(features) = value.get("features") {
        if !features.as_array().is_some_and(|a| {
            a.iter().all(|v| {
                v.as_str().is_some_and(|s| {
                    ["rego_v1", "keywords_in_refs", "template_strings"].contains(&s)
                })
            })
        }) {
            return Err("unsupported OPA capability feature".into());
        }
    }
    Ok(())
}
