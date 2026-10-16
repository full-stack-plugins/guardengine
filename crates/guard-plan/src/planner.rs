use crate::{FrozenPlan, PlanningOutcome, TargetIssue};
use guard_contract::CompiledContract;
use guard_protocol::{
    Digest, ExactTarget, ExecutionBudget, Extensions, Identifier, Obligation, ProtocolDocument,
    SchemaVersion, TaskBinding, VerificationPlan,
};
use guard_subject::CapturedSubject;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

fn json_digest(value: &Value) -> Result<Digest, String> {
    guard_protocol::digest_json(&serde_json::to_vec(value).map_err(|e| e.to_string())?)
}
fn keys<T: serde::Serialize>(values: &[T]) -> Result<BTreeSet<Vec<u8>>, String> {
    let mut result = BTreeSet::new();
    for value in values {
        let encoded =
            guard_protocol::canonicalize(&serde_json::to_vec(value).map_err(|e| e.to_string())?)?;
        if !result.insert(encoded) {
            return Err("duplicate frozen reference or scope item".into());
        }
    }
    Ok(result)
}
/// 计算全部固定验证器/OPA 身份，不接收调用者自造工具清单作为替代。
pub fn verifier_digest(contract: &CompiledContract) -> Result<Digest, String> {
    json_digest(&serde_json::to_value(contract.verifiers()).map_err(|e| e.to_string())?)
}

fn validate_binding(
    contract: &CompiledContract,
    binding: &TaskBinding,
    subject: &CapturedSubject,
) -> Result<(), String> {
    guard_protocol::encode(&ProtocolDocument::TaskBinding(binding.clone()))?;
    let context = contract.context();
    if binding.contract_digest != *contract.digest()
        || binding.task_id != context.task_id
        || binding.scope.repository_ids != [context.repository_id.clone()]
        || binding.scope.actions != [contract.action()]
        || binding.authorization_refs != [contract.authorization_ref().clone()]
        || subject.binding.repository_id != context.repository_id
        || subject.binding.task_id != binding.task_id
        || subject.binding.worktree_id != binding.worktree_id
        || subject.binding.contract_digest != *contract.digest()
        || subject.binding.verifier_digest != verifier_digest(contract)?
    {
        return Err(
            "task, scope, authorization, worktree or pinned verifier binding mismatch".into(),
        );
    }
    let approved_sources = keys(contract.source_refs())?;
    if keys(&binding.requirement_refs)? != approved_sources {
        return Err("task source references differ from frozen approved sources".into());
    }
    let approved_paths: BTreeSet<_> = contract
        .obligations()
        .iter()
        .flat_map(|o| &o.targets)
        .map(|target| target.path.to_bytes())
        .collect();
    let bound_paths: BTreeSet<_> = binding
        .scope
        .paths
        .iter()
        .map(|path| path.to_bytes())
        .collect();
    if bound_paths.len() != binding.scope.paths.len() || bound_paths != approved_paths {
        return Err("task paths must equal approved exact selection paths".into());
    }
    if contract.obligations().is_empty() || contract.verifiers().is_empty() {
        return Err("empty frozen obligations or verifier pins".into());
    }
    Ok(())
}

/// 从不变编译产物、显式任务和真实 Git 采集冻结执行前计划；不接受运行结果作输入。
/// 无保护摘要的目标绑定当前候选字节，候选代码更新不要求修改已批准契约。
/// 缺目标或 protectedDigest 改变时保留全集并返回 Incomplete，不创建局部运行计划。
pub fn freeze(
    project: &Path,
    contract: &CompiledContract,
    binding: &TaskBinding,
    subject: &CapturedSubject,
    budget: ExecutionBudget,
) -> Result<PlanningOutcome, String> {
    if subject.snapshot.as_git().is_none() {
        return Err("release artifact capture/planning provider is not implemented by this Git-only planner".into());
    }
    if budget.timeout_ms == 0
        || budget.timeout_ms > 9007199254740991
        || budget.max_output_bytes == 0
        || budget.max_output_bytes > 9007199254740991
        || budget.max_concurrency == 0
    {
        return Err("positive safe-integer execution budgets required".into());
    }
    validate_binding(contract, binding, subject)?;
    guard_subject::verify_current(project, &subject.binding, &subject.selection, subject)?;
    let manifest: BTreeMap<_, _> = subject
        .manifest
        .iter()
        .map(|entry| (entry.path.to_bytes(), entry))
        .collect();
    if manifest.len() != subject.manifest.len() {
        return Err("duplicate actual manifest target".into());
    }
    let mut issues = Vec::new();
    let mut obligations = Vec::new();
    let mut union = BTreeMap::new();
    for obligation in contract.obligations() {
        let mut targets = Vec::new();
        for selection in &obligation.targets {
            if selection.repository_id != contract.context().repository_id {
                return Err("cross-repository target requires separate captured subject".into());
            }
            let Some(input) = manifest.get(&selection.path.to_bytes()) else {
                issues.push(TargetIssue {
                    obligation_id: obligation.obligation_id.clone(),
                    selection: selection.clone(),
                    code: "missing_input".into(),
                });
                continue;
            };
            if selection
                .protected_digest
                .as_ref()
                .is_some_and(|digest| digest != &input.content_digest)
            {
                issues.push(TargetIssue {
                    obligation_id: obligation.obligation_id.clone(),
                    selection: selection.clone(),
                    code: "protected_input_changed".into(),
                });
                continue;
            }
            let target = ExactTarget {
                repository_id: selection.repository_id.clone(),
                path: selection.path.clone(),
                selector: selection.selector.clone(),
                input_digest: input.content_digest.clone(),
            };
            union.insert(
                guard_protocol::canonicalize(
                    &serde_json::to_vec(&target).map_err(|e| e.to_string())?,
                )?,
                target.clone(),
            );
            targets.push(target);
        }
        obligations.push(Obligation {
            obligation_id: obligation.obligation_id.clone(),
            rule_id: obligation.rule_id.clone(),
            verifier_id: obligation.verifier_id.clone(),
            required: obligation.required,
            targets,
            required_evidence: obligation.required_evidence.clone(),
        });
    }
    let approvals: Vec<_> = contract.approval_attestations().iter().map(|approval| json!({
        "authorityId":approval.authority_id,"approvalRef":approval.approval_ref,"contractDigest":approval.contract_digest,
        "context":approval.context,"level":approval.level,"action":approval.action})).collect();
    let contract_value = json!({"digest":contract.digest(),"context":contract.context(),"action":contract.action(),
        "layers":contract.layers(),"rules":contract.rules(),"obligations":contract.obligations(),"verifiers":contract.verifiers(),
        "authorizationRef":contract.authorization_ref(),"authorizationAuthority":contract.authorization_authority(),
        "schemaDigest":contract.schema_digest(),"sourceRefs":contract.source_refs(),"approvals":approvals});
    let baseline = json!({"schemaVersion":"guard.task-baseline/v1","contract":contract_value,"taskBinding":binding});
    let seed = json!({"schemaVersion":"guard.frozen-binding/v1","contract":contract_value,
        "taskBinding":binding,"capturedSubject":subject,"budget":budget});
    let plan = if issues.is_empty() {
        let seed_digest = json_digest(&seed)?;
        let verification = VerificationPlan {
            schema_version: SchemaVersion::new("1.0.0")?,
            plan_id: Identifier::new(format!("plan:{}", &seed_digest.as_str()[7..]))?,
            subject_digest: subject.identity_digest.clone(),
            contract_digest: contract.digest().clone(),
            policy_digest: subject.binding.policy_digest.clone(),
            action: contract.action(),
            obligations,
            targets: union.into_values().collect(),
            budget,
            extensions: Extensions::default(),
        };
        let digest = guard_protocol::digest_json(&guard_protocol::encode(
            &ProtocolDocument::VerificationPlan(verification.clone()),
        )?)?;
        Some(FrozenPlan {
            verification,
            digest,
        })
    } else {
        None
    };
    // 不完整结果仍保留全部批准义务和真实输入，不暴露删目标后的临时运行向量。
    let snapshot = json!({"frozen":seed,"issues":issues,"verificationPlan":plan.as_ref().map(|p| &p.verification),"planDigest":plan.as_ref().map(|p| &p.digest)});
    let digest = json_digest(&snapshot)?;
    Ok(PlanningOutcome {
        contract: contract.clone(),
        binding: binding.clone(),
        plan,
        issues,
        snapshot,
        digest,
        subject: subject.clone(),
        baseline,
        project: project.canonicalize().map_err(|e| e.to_string())?,
    })
}
