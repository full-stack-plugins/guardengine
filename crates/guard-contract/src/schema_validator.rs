use crate::{SchemaIssue, deny_external::DenyExternal};
use guard_protocol::Digest;
use serde_json::Value;
use sha2::{Digest as Hasher, Sha256};
/// 真正的 Draft2020-12 求值器；不访问外部网络、文件或执行工具。无 Java 对应类型。
pub struct SchemaValidator {
    validator: jsonschema::Validator,
    digest: Digest,
}
impl SchemaValidator {
    /// 由原始字节构建求值器；拒绝重复键、外部检索和未知 draft。
    pub fn build(bytes: &[u8]) -> Result<Self, String> {
        let canonical = guard_protocol::canonicalize(bytes)?;
        let schema: Value = serde_json::from_slice(&canonical).map_err(|e| e.to_string())?;
        crate::schema_policy::check_schema_policy(&schema)?;
        jsonschema::draft202012::meta::validate(&schema)
            .map_err(|_| "invalid Draft2020-12 Schema".to_string())?;
        let validator = jsonschema::draft202012::options()
            .should_validate_formats(true)
            .should_ignore_unknown_formats(false)
            .with_retriever(DenyExternal)
            .build(&schema)
            .map_err(|_| "Schema compilation failed".to_string())?;
        let digest = Digest::new(format!("sha256:{:x}", Sha256::digest(bytes)))?;
        Ok(Self { validator, digest })
    }
    /// 求值实例并给出最多64个错误位置；空列表仅表示该结构 Schema 满足。
    pub fn validate(&self, value: &Value) -> Vec<SchemaIssue> {
        self.validator
            .iter_errors(value)
            .take(64)
            .map(|e| SchemaIssue {
                instance_path: e.instance_path.to_string(),
                schema_path: e.schema_path.to_string(),
            })
            .collect()
    }
    /// 对原始输入执行严格 JSON 和真实 Schema 检查，不提前折叠重复键。
    pub fn validate_bytes(&self, bytes: &[u8]) -> Result<Vec<SchemaIssue>, String> {
        let bytes = guard_protocol::canonicalize(bytes)?;
        let value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        Ok(self.validate(&value))
    }
    /// 返回本次实际使用 Schema 原始字节的摘要。
    pub fn digest(&self) -> &Digest {
        &self.digest
    }
    /// 使用随 guard-protocol 构建冻结的闭合 Schema，仍要另行验证协议语义。
    pub fn protocol() -> Result<Self, String> {
        Self::build(include_bytes!(
            "../../guard-protocol/schemas/engineering.schema.json"
        ))
    }
}
