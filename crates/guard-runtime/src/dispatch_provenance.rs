use crate::{ArtifactRef, SourceInvocation};
use guard_protocol::{
    Digest, DomainOwner, ExecutionStatus, Identifier, ProducerIdentity, RunSequence, SubjectKind,
    UtcTimestamp,
};
use guard_run_ledger::{RunIdentity, RunScope, RunState};
use serde::{Deserialize, Serialize};
/// 真实派发路径冻结的机械来源数据；可解析声明不等于完成凭据，不含质量结论。无Java对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DispatchProvenance {
    pub schema_version: String,
    pub scope: RunScope,
    pub identity: RunIdentity,
    pub plan_digest: Digest,
    pub plan_head_digest: Digest,
    pub run_head_digest: Digest,
    pub operation_id: Identifier,
    pub sequence: RunSequence,
    pub subject_kind: SubjectKind,
    pub verifier_id: Identifier,
    pub producer: ProducerIdentity,
    pub domain: DomainOwner,
    pub verifier_pin_digest: Digest,
    pub profile: String,
    pub approved_invocation: SourceInvocation,
    pub effective_invocation: SourceInvocation,
    pub execution_status: ExecutionStatus,
    pub state: RunState,
    pub exit_code: Option<i32>,
    pub started_at: Option<UtcTimestamp>,
    pub finished_at: Option<UtcTimestamp>,
    pub output_truncated: bool,
    pub input_drifted: bool,
    pub metadata: ArtifactRef,
    pub stdout: ArtifactRef,
    pub stderr: ArtifactRef,
}

impl DispatchProvenance {
    /// 校验已签声明所指的当前记录；只读检查绝不恢复DispatchCompletion或授信。
    pub fn verify_recorded_current(
        &self,
        plan: &guard_plan::RegisteredPlan,
        contract: &guard_contract::CompiledContract,
        store: &guard_store::EvidenceStore,
        project: &std::path::Path,
        now_ms: u64,
    ) -> Result<(), String> {
        use crate::native_verifier_command::digest;
        use serde_json::json;
        plan.assert_current(store, project, now_ms)?;
        let verification = plan.plan().verification();
        let obligation = verification
            .obligations
            .iter()
            .find(|value| value.obligation_id == self.scope.obligation_id)
            .ok_or("unknown source obligation")?;
        let pin = contract
            .verifiers()
            .iter()
            .find(|value| value.verifier_id == obligation.verifier_id)
            .ok_or("source verifier missing")?;
        let identity = guard_run_ledger::RunIdentity {
            binding_digest: digest(
                &json!({"taskBinding":plan.binding(),"planDigest":plan.plan().digest(),"activeHead":plan.head()}),
            )?,
            subject_digest: verification.subject_digest.clone(),
            contract_digest: verification.contract_digest.clone(),
            policy_digest: verification.policy_digest.clone(),
            verifier_digest: guard_plan::verifier_digest(contract)?,
            environment_digest: plan.capture_binding().environment_digest.clone(),
        };
        let scope = guard_run_ledger::RunScope {
            repository_id: contract.context().repository_id.clone(),
            task_id: plan.binding().task_id.clone(),
            obligation_id: obligation.obligation_id.clone(),
            approved_scope_digest: digest(&plan.binding().scope)?,
        };
        if self.schema_version != "guard.dispatch-provenance/v1"
            || self.identity != identity
            || self.scope != scope
            || self.plan_digest != *plan.plan().digest()
            || self.plan_head_digest != digest(plan.head())?
            || self.subject_kind
                != plan
                    .captured_subject()
                    .snapshot
                    .as_git()
                    .ok_or("Git source required")?
                    .kind
            || self.verifier_id != pin.verifier_id
            || self.verifier_pin_digest != digest(pin)?
            || Some(self.domain) != pin.owner
            || self.producer.producer_id != pin.verifier_id
            || self.producer.version != pin.version
            || self.producer.artifact_digest != pin.artifact_digest
            || self.producer.config_digest != pin.config_digest
            || self.producer.adapter_version != pin.adapter_version
            || self.producer.execution_environment_digest != identity.environment_digest
            || verification.contract_digest != *contract.digest()
        {
            return Err("source declaration differs from registered plan/pins".into());
        }
        self.approved_invocation
            .verify_effective(&self.effective_invocation, &self.profile)?;
        let ledger = guard_run_ledger::RunLedger::new(store);
        let current = ledger.current_receipt(&scope, &identity, now_ms)?;
        if self.run_head_digest != digest(current.head())?
            || self.sequence != *current.sequence()
            || self.operation_id != *current.operation_id()
            || self.state != current.state()
            || self.metadata.operation != current.metadata_operation()
            || &self.metadata.digest != current.metadata_digest()
        {
            return Err("source run head/metadata no longer current".into());
        }
        for artifact in [&self.metadata, &self.stdout, &self.stderr] {
            let bytes = store.read_private(&artifact.operation, now_ms)?;
            let summary = store.summary(&artifact.operation)?;
            if summary.content_digest != artifact.digest
                || summary.size != artifact.size
                || bytes.len() as u64 != artifact.size
            {
                return Err("source artifact missing or mismatched".into());
            }
        }
        let raw = store.read_private(&self.metadata.operation, now_ms)?;
        let metadata: serde_json::Value =
            serde_json::from_slice(&guard_protocol::canonicalize(&raw)?)
                .map_err(|e| e.to_string())?;
        for (field, expected) in [
            ("schemaVersion", json!("guard.execution-observation/v1")),
            ("obligationId", json!(self.scope.obligation_id)),
            ("verifierId", json!(self.verifier_id)),
            ("identity", json!(self.identity)),
            ("planDigest", json!(self.plan_digest)),
            ("sequence", json!(self.sequence)),
            ("executionStatus", json!(self.execution_status)),
            ("state", json!(self.state)),
            ("exitCode", json!(self.exit_code)),
            ("startedAt", json!(self.started_at)),
            ("finishedAt", json!(self.finished_at)),
            ("outputTruncated", json!(self.output_truncated)),
            ("inputDrifted", json!(self.input_drifted)),
            ("stdout", json!(self.stdout)),
            ("stderr", json!(self.stderr)),
            ("trust", json!("local_unverified")),
        ] {
            if metadata.get(field) != Some(&expected) {
                return Err(format!("source metadata field mismatch: {field}"));
            }
        }
        if metadata["invocation"]["profile"] != self.profile
            || metadata["invocation"]["approvedConfigDigest"] != json!(self.producer.config_digest)
            || metadata["invocation"]["subjectDigest"] != json!(self.identity.subject_digest)
            || metadata["invocation"]["actualEnvironmentDigest"]
                != json!(digest(&self.effective_invocation.environment)?)
            || metadata["invocation"]["effectiveInvocationDigest"]
                != json!(digest(
                    &json!({"schemaVersion":"guard.effective-invocation/v1","executable":self.effective_invocation.executable,"artifactDigest":self.effective_invocation.artifact_digest,"argv":self.effective_invocation.argv,"workingDirectory":self.effective_invocation.working_directory,"environment":self.effective_invocation.environment})
                )?)
        {
            return Err("source invocation metadata mismatch".into());
        }
        ledger.assert_current_receipt(&current, now_ms)?;
        plan.assert_current(store, project, now_ms)
    }
}
