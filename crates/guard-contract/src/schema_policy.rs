use serde_json::Value;

// 仅沿 Draft2020-12 定义的子 Schema 位置检查，保留 const/default/examples 中的数据原样。
pub(crate) fn check_schema_policy(schema: &Value) -> Result<(), String> {
    if let Some(draft) = schema.get("$schema") {
        if draft != "https://json-schema.org/draft/2020-12/schema" {
            return Err("only Draft2020-12 is supported".into());
        }
    }
    if let Some(vocabulary) = schema.get("$vocabulary").and_then(Value::as_object) {
        for (uri, required) in vocabulary {
            let known = [
                "core",
                "applicator",
                "unevaluated",
                "validation",
                "meta-data",
                "format-annotation",
                "format-assertion",
                "content",
            ]
            .iter()
            .any(|name| uri == &format!("https://json-schema.org/draft/2020-12/vocab/{name}"));
            if required == true && !known {
                return Err("unsupported required Schema vocabulary".into());
            }
        }
    }
    for keyword in [
        "$defs",
        "properties",
        "patternProperties",
        "dependentSchemas",
    ] {
        if let Some(children) = schema.get(keyword).and_then(Value::as_object) {
            for child in children.values() {
                check_schema_policy(child)?;
            }
        }
    }
    for keyword in ["allOf", "anyOf", "oneOf", "prefixItems"] {
        if let Some(children) = schema.get(keyword).and_then(Value::as_array) {
            for child in children {
                check_schema_policy(child)?;
            }
        }
    }
    for keyword in [
        "not",
        "if",
        "then",
        "else",
        "items",
        "contains",
        "additionalProperties",
        "propertyNames",
        "unevaluatedItems",
        "unevaluatedProperties",
        "contentSchema",
    ] {
        if let Some(child) = schema.get(keyword) {
            check_schema_policy(child)?;
        }
    }
    Ok(())
}
