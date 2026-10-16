use crate::{
    SchemaValidator, VerifierDescriptor, VerifierKind, VerifierPin,
    registered_verifier::RegisteredVerifier,
};
use guard_protocol::Identifier;
use std::collections::{BTreeMap, BTreeSet};
/// 调用方受信配置构造的只读固定验证器注册表；候选规则不能覆盖或注入描述。无 Java 对应类型。
pub struct VerifierRegistry {
    pub(crate) policy_engine_id: Identifier,
    pub(crate) entries: BTreeMap<Identifier, RegisteredVerifier>,
}
impl VerifierRegistry {
    /// 固定必需策略引擎ID；不存在时编译保守阻塞而不执行安装。
    pub fn new(policy_engine_id: Identifier) -> Self {
        Self {
            policy_engine_id,
            entries: BTreeMap::new(),
        }
    }
    /// 注册固定描述及真实参数Schema；同ID不同身份拒绝，不原地升级旧描述。
    pub fn register(&mut self, descriptor: VerifierDescriptor) -> Result<(), String> {
        fn fixed(value: &str) -> bool {
            !matches!(
                value.to_ascii_lowercase().as_str(),
                "latest" | "main" | "master" | "head"
            ) && !value
                .chars()
                .any(|c| c.is_whitespace() || matches!(c, '*' | '^' | '~' | '<' | '>' | '='))
                && !value.split('.').any(|s| s.eq_ignore_ascii_case("x"))
        }
        if !fixed(descriptor.version.as_str()) || !fixed(descriptor.adapter_version.as_str()) {
            return Err("verifier version must be pinned literal".into());
        }
        if (descriptor.kind == VerifierKind::Domain) != descriptor.owner.is_some() {
            return Err("verifier owner disagrees with kind".into());
        }
        if descriptor.kind == VerifierKind::PolicyEngine
            && descriptor.verifier_id != self.policy_engine_id
        {
            return Err("policy engine ID is not configured".into());
        }
        let mut evidence = BTreeSet::new();
        if descriptor.evidence_types.is_empty()
            || !descriptor
                .evidence_types
                .iter()
                .all(|e| evidence.insert(e.clone()))
        {
            return Err("verifier needs unique evidence types".into());
        }
        let parameters = SchemaValidator::build(&descriptor.parameter_schema)?;
        let pin = VerifierPin {
            verifier_id: descriptor.verifier_id,
            version: descriptor.version,
            artifact_digest: descriptor.artifact_digest,
            config_digest: descriptor.config_digest,
            adapter_version: descriptor.adapter_version,
            kind: descriptor.kind,
            owner: descriptor.owner,
            evidence_types: descriptor.evidence_types,
            parameter_schema_digest: parameters.digest().clone(),
        };
        if let Some(existing) = self.entries.get(&pin.verifier_id) {
            return if existing.pin == pin {
                Ok(())
            } else {
                Err("conflicting verifier identity for existing ID".into())
            };
        }
        self.entries.insert(
            pin.verifier_id.clone(),
            RegisteredVerifier { pin, parameters },
        );
        Ok(())
    }
    /// 返回固定身份，观测端必须逐字段比对而不能只检查ID或工具存在。
    pub fn pin(&self, id: &Identifier) -> Option<&VerifierPin> {
        self.entries.get(id).map(|v| &v.pin)
    }
}
