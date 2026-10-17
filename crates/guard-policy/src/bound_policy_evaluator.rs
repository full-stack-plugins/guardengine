use crate::{OpaConfig, OpaEvaluator, PolicyBinding, PolicyInput, PolicyPackage, PolicyReceipt};
use guard_contract::{CompiledContract, VerifierKind};
use guard_plan::RegisteredPlan;
use guard_runner::Cancellation;
use guard_store::EvidenceStore;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
/// 与真实注册计划及批准OPA pin绑定的求值器；不是生产receipt权威或授权签发者。无Java对应类型。
#[derive(Clone)]
pub struct BoundPolicyEvaluator {
    inner: OpaEvaluator,
    plan: RegisteredPlan,
    contract: CompiledContract,
    project: PathBuf,
}
impl BoundPolicyEvaluator {
    /// 同步只读准备：核当前候选/active head及编译pin；异步宿主须在有界blocking线程调用。
    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        project: &Path,
        contract: &CompiledContract,
        plan: &RegisteredPlan,
        config: OpaConfig,
        package: PolicyPackage,
        store: Arc<EvidenceStore>,
        scratch: &Path,
        retain: u64,
    ) -> Result<Self, String> {
        let project = crate::paths::safe(project)?;
        let scratch = crate::paths::safe(scratch)?;
        if scratch.starts_with(&project) {
            return Err("policy scratch must be outside candidate".into());
        }
        plan.assert_current(&store, &project, crate::opa_evaluator::now()?)?;
        let policies = contract
            .verifiers()
            .iter()
            .filter(|p| p.kind == VerifierKind::PolicyEngine)
            .collect::<Vec<_>>();
        if policies.len() != 1 {
            return Err("exactly one approved OPA policy engine required".into());
        }
        let pin = policies[0];
        if contract.digest() != &plan.plan().verification().contract_digest
            || package.digest() != &plan.plan().verification().policy_digest
            || pin.artifact_digest != config.executable_digest
            || pin.version != config.version
            || pin.adapter_version.as_str() != "guard-opa/v1"
            || pin.config_digest != config.configuration_digest(&package)?
            || config.budget.timeout_ms > plan.plan().verification().budget.timeout_ms
            || config.budget.max_concurrency > plan.plan().verification().budget.max_concurrency
            || config.budget.max_output_bytes > plan.plan().verification().budget.max_output_bytes
        {
            return Err("policy package/version/adapter/config/contract or budget differs from approved frozen plan".into());
        }
        Ok(Self {
            inner: OpaEvaluator::new(config, package, store, scratch.as_path(), retain)?,
            plan: plan.clone(),
            contract: contract.clone(),
            project,
        })
    }
    /// 返回真实计划派生的固定binding；不从领域facts读取task/subject/contract身份。
    pub fn binding(&self) -> PolicyBinding {
        PolicyBinding {
            repository_id: self.contract.context().repository_id.clone(),
            task_id: self.plan.binding().task_id.clone(),
            subject_digest: self.plan.plan().verification().subject_digest.clone(),
            contract_digest: self.contract.digest().clone(),
            policy_digest: self.inner.package.digest().clone(),
            plan_digest: self.plan.plan().digest().clone(),
            action: self.contract.action(),
        }
    }
    async fn check_current(&self) -> Result<(), String> {
        let plan = self.plan.clone();
        let project = self.project.clone();
        let store = self.inner.store.clone();
        crate::blocking::run(move || {
            plan.assert_current(&store, &project, crate::opa_evaluator::now()?)
        })
        .await
    }
    /// 核验当前对象后执行OPA，结束再次核验；漂移保留原始输出但撤销可用策略结论。
    pub async fn evaluate(
        &self,
        facts: &[u8],
        operation: &str,
        cancel: Cancellation,
    ) -> Result<PolicyReceipt, String> {
        let deadline = std::time::Instant::now()
            + std::time::Duration::from_millis(self.inner.config.budget.timeout_ms);
        let plan = self.plan.clone();
        let project = self.project.clone();
        let store = self.inner.store.clone();
        if let Err(error) = crate::blocking::run_until(deadline, move || {
            plan.assert_current(&store, &project, crate::opa_evaluator::now()?)
        })
        .await
        {
            if std::time::Instant::now() < deadline {
                return Err(error);
            }
        }
        let input = PolicyInput::from_facts(self.binding(), facts)?;
        let mut receipt = self
            .inner
            .evaluate_until(&input, operation, cancel, deadline)
            .await?;
        if let Err(error) = self.check_current().await {
            receipt.evaluation.incomplete = true;
            receipt.evaluation.decision = None;
            receipt
                .evaluation
                .diagnostics
                .push(format!("registered policy context changed: {error}"));
            let bytes = guard_protocol::canonicalize(
                &serde_json::to_vec(&receipt.evaluation).map_err(|e| e.to_string())?,
            )?;
            receipt.artifact = crate::opa_evaluator::save(
                &self.inner.store,
                &format!("{operation}:postcheck-metadata"),
                &bytes,
                self.inner.retain(),
            )?;
        }
        Ok(receipt)
    }
}
