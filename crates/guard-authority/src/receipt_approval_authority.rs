use crate::{
    ApprovalMaterial, ApprovalPurpose, ApprovalVerifier, ApprovedScope, VerificationRequest,
};
use guard_contract::{
    ApprovalAttestation, ApprovalAuthority, CompileContext, CompileRequest, LayerLevel,
    TaskAuthorization, snapshot_digest,
};
use guard_plan::{MigrationAttestation, MigrationAuthority, MigrationRequest};
use guard_protocol::{Action, ApprovalIntent, Digest, Identifier, ProtocolDocument};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
/// 将真实CompileRequest的完整目标冻结后接现有批准端口，不从候选传入较小scope。无Java对应类型。
pub struct ReceiptApprovalAuthority {
    verifier: Arc<ApprovalVerifier>,
    context: CompileContext,
    action: Action,
    scope: ApprovedScope,
    audience: Identifier,
    requested_by: Identifier,
    layers: BTreeMap<LayerLevel, (Identifier, Digest)>,
    migration_ref: Option<Identifier>,
}
impl ReceiptApprovalAuthority {
    /// 从真实三层请求提取完整目标全集和层摘要；批准仍由外部源显式提供。
    pub fn for_compile(
        request: &CompileRequest,
        verifier: Arc<ApprovalVerifier>,
        audience: Identifier,
        requested_by: Identifier,
        migration_ref: Option<Identifier>,
    ) -> Result<Self, String> {
        if request.layers.len() != 3
            || request.context.authorization_refs.is_empty()
            || request.context.authorization_refs.len() > 128
        {
            return Err("three layers and bounded authorization refs required".into());
        }
        let mut layers = BTreeMap::new();
        let mut targets = BTreeMap::new();
        for layer in &request.layers {
            let decoded = guard_protocol::decode(&layer.document)?;
            let ProtocolDocument::ContractSnapshot(snapshot) = decoded.document() else {
                return Err("ContractSnapshot required".into());
            };
            let digest = snapshot_digest(snapshot)?;
            if digest != snapshot.digest
                || layers
                    .insert(layer.level, (snapshot.approval_ref.clone(), digest))
                    .is_some()
            {
                return Err("duplicate layer or wrong snapshot digest".into());
            }
            for obligation in &snapshot.obligations {
                for target in &obligation.targets {
                    let bytes = guard_protocol::canonicalize(
                        &serde_json::to_vec(target).map_err(|e| e.to_string())?,
                    )?;
                    targets.insert(bytes, target.clone());
                }
            }
        }
        if layers.len() != 3 {
            return Err("organization/project/task layers required".into());
        }
        let scope = ApprovedScope {
            repository_id: request.context.repository_id.clone(),
            task_id: request.context.task_id.clone(),
            actions: vec![request.action],
            targets: targets.into_values().collect(),
        };
        scope.digest()?;
        Ok(Self {
            verifier,
            context: request.context.clone(),
            action: request.action,
            scope,
            audience,
            requested_by,
            layers,
            migration_ref,
        })
    }
    /// 返回从真实输入推导的精确scope，仅供受保护批准端展示与签名，不代表已批准。
    pub fn requested_scope(&self) -> &ApprovedScope {
        &self.scope
    }
    fn request(
        &self,
        reference: &Identifier,
        digest: &Digest,
        purpose: ApprovalPurpose,
    ) -> VerificationRequest {
        VerificationRequest {
            reference: reference.clone(),
            material: ApprovalMaterial {
                schema_version: "guard.approval-material/v1".into(),
                audience: self.audience.clone(),
                requested_by: self.requested_by.clone(),
                action: self.action,
                purpose,
                scope: self.scope.clone(),
            },
            contract_digest: digest.clone(),
            intent: ApprovalIntent::RequirementScope,
            subject_digest: None,
        }
    }
    fn context_matches(&self, context: &CompileContext, action: Action) -> bool {
        &self.context == context && self.action == action
    }
}
impl ApprovalAuthority for ReceiptApprovalAuthority {
    fn verify_layer(
        &self,
        context: &CompileContext,
        level: LayerLevel,
        reference: &Identifier,
        digest: &Digest,
        action: Action,
    ) -> Result<ApprovalAttestation, String> {
        if !self.context_matches(context, action)
            || self.layers.get(&level) != Some(&(reference.clone(), digest.clone()))
        {
            return Err("layer request differs from frozen actual inputs".into());
        }
        let verified = self.verifier.verify(&self.request(
            reference,
            digest,
            ApprovalPurpose::Layer {
                level: level.into(),
            },
        ))?;
        Ok(ApprovalAttestation {
            authority_id: verified.issuer().clone(),
            approval_ref: reference.clone(),
            contract_digest: digest.clone(),
            context: context.clone(),
            level,
            action,
        })
    }
    fn authorize_task(
        &self,
        context: &CompileContext,
        digest: &Digest,
        action: Action,
    ) -> Result<TaskAuthorization, String> {
        if !self.context_matches(context, action) {
            return Err("task request differs from frozen actual inputs".into());
        }
        for reference in &context.authorization_refs {
            if let Ok(verified) =
                self.verifier
                    .verify(&self.request(reference, digest, ApprovalPurpose::Task))
            {
                return Ok(TaskAuthorization {
                    authority_id: verified.issuer().clone(),
                    authorization_ref: reference.clone(),
                    contract_digest: digest.clone(),
                    context: context.clone(),
                    action,
                });
            }
        }
        Err("no current independent task authorization".into())
    }
}
impl MigrationAuthority for ReceiptApprovalAuthority {
    fn verify(&self, request: &MigrationRequest) -> Result<MigrationAttestation, String> {
        let next = request.next_binding();
        let paths = self
            .scope
            .targets
            .iter()
            .map(|target| target.path.to_bytes())
            .collect::<BTreeSet<_>>();
        let actual = next
            .scope
            .paths
            .iter()
            .map(|path| path.to_bytes())
            .collect::<BTreeSet<_>>();
        if request.task_id() != &self.context.task_id
            || next.task_id != self.context.task_id
            || next.scope.repository_ids != [self.context.repository_id.clone()]
            || actual != paths
            || next.scope.paths.len() != actual.len()
            || next.scope.actions != self.scope.actions
            || next.authorization_refs != self.context.authorization_refs
        {
            return Err("migration next scope differs from frozen actual contract".into());
        }
        let reference = self
            .migration_ref
            .as_ref()
            .ok_or("independent migration approval reference missing")?;
        let verified = self.verifier.verify(&self.request(
            reference,
            &next.contract_digest,
            ApprovalPurpose::Migration {
                request_digest: request.digest().clone(),
            },
        ))?;
        Ok(MigrationAttestation {
            authority_id: verified.issuer().clone(),
            approval_ref: reference.clone(),
            request_digest: request.digest().clone(),
        })
    }
}
