use crate::{
    host_config::HostConfig,
    safe_files::{decode, get},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use guard_authority::{
    AnchorSource, ApprovalPermission, AuthorityAnchor, AuthorityClock, PrincipalRole,
    ReceiptBundle, ReceiptSource, RevocationSnapshot, RevocationSource,
};
use guard_protocol::{Identifier, UtcTimestamp};
use serde_json::Value;
/// 只读取固定宿主根原件的真实端口；保护与当前撤销发布由部署者负责。无Java对应类型。
#[derive(Clone)]
pub(crate) struct FilePorts(pub HostConfig);
impl AuthorityClock for FilePorts {
    fn now(&self) -> Result<UtcTimestamp, String> {
        UtcTimestamp::new(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
    }
}
impl ReceiptSource for FilePorts {
    fn get(&self, reference: &Identifier) -> Result<ReceiptBundle, String> {
        self.0.reload_same_profile()?;
        let files = self.0.data["receipts"]
            .get(reference.as_str())
            .ok_or("receipt reference is not configured by host")?;
        let object = files.as_object().ok_or("receipt file pair required")?;
        if object.len() != 2 || !object.contains_key("receipt") || !object.contains_key("material")
        {
            return Err("closed receipt file pair required".into());
        }
        Ok(ReceiptBundle {
            receipt: self.0.file(&get::<String>(files, "receipt")?)?,
            material: self.0.file(&get::<String>(files, "material")?)?,
        })
    }
}
impl AnchorSource for FilePorts {
    fn current(&self, key_id: &Identifier) -> Result<AuthorityAnchor, String> {
        // 每次重读host.json，不缓存被撤销的信任锚。
        let current = self.0.reload_same_profile()?;
        let anchors: Vec<Value> = current.field("anchors")?;
        let matches = anchors
            .iter()
            .filter(|v| v["keyId"] == key_id.as_str())
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err("unique independent key anchor required".into());
        }
        let value = matches[0];
        closed(
            value,
            &[
                "keyId",
                "issuer",
                "publicKey",
                "revocationAuthority",
                "identities",
                "audiences",
                "intents",
            ],
        )?;
        let raw = STANDARD
            .decode(get::<String>(value, "publicKey")?)
            .map_err(|e| e.to_string())?;
        let public_key: [u8; 32] = raw
            .try_into()
            .map_err(|_| "Ed25519 key must have 32 bytes")?;
        let mut identities = Vec::new();
        for identity in get::<Vec<Value>>(value, "identities")? {
            closed(&identity, &["principal", "role", "purposes", "intents"])?;
            let purposes = get::<Vec<String>>(&identity, "purposes")?
                .iter()
                .map(|p| match p.as_str() {
                    "organization_layer" => Ok(ApprovalPermission::OrganizationLayer),
                    "project_layer" => Ok(ApprovalPermission::ProjectLayer),
                    "task_layer" => Ok(ApprovalPermission::TaskLayer),
                    "task" => Ok(ApprovalPermission::Task),
                    "migration" => Ok(ApprovalPermission::Migration),
                    _ => Err("unknown independent role purpose".to_string()),
                })
                .collect::<Result<Vec<_>, _>>()?;
            identities.push(PrincipalRole {
                principal: get(&identity, "principal")?,
                role: get(&identity, "role")?,
                purposes,
                intents: get(&identity, "intents")?,
            });
        }
        Ok(AuthorityAnchor {
            key_id: get(value, "keyId")?,
            issuer: get(value, "issuer")?,
            public_key,
            revocation_authority: get(value, "revocationAuthority")?,
            identities,
            audiences: get(value, "audiences")?,
            intents: get(value, "intents")?,
        })
    }
}
impl RevocationSource for FilePorts {
    fn current(
        &self,
        issuer: &Identifier,
        key: &Identifier,
        receipt: &Identifier,
        reference: &Identifier,
    ) -> Result<RevocationSnapshot, String> {
        let current = self.0.reload_same_profile()?;
        let file = current.data["revocations"]
            .get(receipt.as_str())
            .and_then(Value::as_str)
            .ok_or("current independent revocation entry unavailable")?;
        let value: Value = decode(&current.file(file)?)?;
        closed(
            &value,
            &[
                "authority",
                "issuer",
                "keyId",
                "receiptId",
                "revocationRef",
                "revision",
                "stateDigest",
                "validFrom",
                "expiresAt",
                "revoked",
            ],
        )?;
        let result = RevocationSnapshot {
            authority: get(&value, "authority")?,
            issuer: get(&value, "issuer")?,
            key_id: get(&value, "keyId")?,
            receipt_id: get(&value, "receiptId")?,
            revocation_ref: get(&value, "revocationRef")?,
            revision: get(&value, "revision")?,
            state_digest: get(&value, "stateDigest")?,
            valid_from: get(&value, "validFrom")?,
            expires_at: get(&value, "expiresAt")?,
            revoked: get(&value, "revoked")?,
        };
        if &result.issuer != issuer
            || &result.key_id != key
            || &result.receipt_id != receipt
            || &result.revocation_ref != reference
        {
            return Err("revocation entry does not match requested identity".into());
        }
        Ok(result)
    }
}
impl guard_authority::ProducerRegistry for FilePorts {
    fn current(
        &self,
        producer: &Identifier,
        audience: &Identifier,
        repository: &Identifier,
        task: &Identifier,
    ) -> Result<guard_authority::ProducerRegistration, String> {
        let current = self.0.reload_same_profile()?;
        let rows: Vec<Value> = current.field("producers")?;
        let matches = rows
            .iter()
            .filter(|v| {
                v["producer"]["producerId"] == producer.as_str()
                    && v["audience"] == audience.as_str()
                    && v["repositoryId"] == repository.as_str()
                    && v["taskId"] == task.as_str()
            })
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err("unique independent producer registration required".into());
        }
        let value = matches[0];
        closed(
            value,
            &[
                "issuer",
                "keyId",
                "publicKey",
                "revocationAuthority",
                "revocationRef",
                "audience",
                "repositoryId",
                "taskId",
                "contractDigest",
                "policyDigest",
                "scopeDigest",
                "producer",
                "domain",
                "verifierPinDigest",
                "profile",
                "approvedInvocation",
                "allowedSubjectKinds",
                "maxValidityMs",
            ],
        )?;
        let public_key = STANDARD
            .decode(get::<String>(value, "publicKey")?)
            .map_err(|e| e.to_string())?
            .try_into()
            .map_err(|_| "32-byte producer public key required")?;
        Ok(guard_authority::ProducerRegistration {
            issuer: get(value, "issuer")?,
            key_id: get(value, "keyId")?,
            public_key,
            revocation_authority: get(value, "revocationAuthority")?,
            revocation_ref: get(value, "revocationRef")?,
            audience: get(value, "audience")?,
            repository_id: get(value, "repositoryId")?,
            task_id: get(value, "taskId")?,
            contract_digest: get(value, "contractDigest")?,
            policy_digest: get(value, "policyDigest")?,
            scope_digest: get(value, "scopeDigest")?,
            producer: get(value, "producer")?,
            domain: get(value, "domain")?,
            verifier_pin_digest: get(value, "verifierPinDigest")?,
            profile: get(value, "profile")?,
            approved_invocation: get(value, "approvedInvocation")?,
            allowed_subject_kinds: get(value, "allowedSubjectKinds")?,
            max_validity_ms: get(value, "maxValidityMs")?,
        })
    }
}
/// 检查对象字段全集精确匹配，返回闭合校验结果。
pub(crate) fn closed(value: &Value, keys: &[&str]) -> Result<(), String> {
    let object = value.as_object().ok_or("object required")?;
    if object.len() != keys.len() || object.keys().any(|k| !keys.contains(&k.as_str())) {
        return Err("closed host record required".into());
    }
    Ok(())
}
