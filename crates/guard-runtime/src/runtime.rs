use crate::check_failure::CheckFailure;
use crate::native_verifier_command::digest;
use crate::{
    ArtifactRef, DispatchOutcome, ExecutionObservation, InvocationReceipt, MaterializationReceipt,
    NativeVerifierCommand, NativeVerifierCommandV2,
};
use guard_contract::CompiledContract;
use guard_plan::RegisteredPlan;
use guard_protocol::{Digest, ExecutionStatus, Identifier, SubjectKind};
use guard_run_ledger::{RunIdentity, RunLedger, RunScope, RunState, StartOutcome};
use guard_runner::{Cancellation, ProcessRunner, StopReason};
use guard_store::EvidenceStore;
use guard_subject::MaterializedSubject;
use serde_json::json;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, OnceLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::Semaphore;

async fn cancelled(cancel: &Cancellation) {
    while !cancel.is_cancelled() {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

fn now() -> Result<u64, String> {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis(),
    )
    .map_err(|e| e.to_string())
}
/// 同一冻结计划共享并发预算的机械协调器；不授予执行许可。无Java对应类型。
#[derive(Clone)]
pub struct Runtime {
    plan: RegisteredPlan,
    contract: CompiledContract,
    store: Arc<EvidenceStore>,
    slots: Arc<Semaphore>,
    project: PathBuf,
    retain_until_ms: u64,
    runner: ProcessRunner,
    scratch_parent: Option<PathBuf>,
}
impl Runtime {
    /// 核验注册计划与编译身份并固定共享预算；时间来自本机墙钟，不是可信时间源。
    pub fn new(
        plan: &RegisteredPlan,
        contract: &CompiledContract,
        store: Arc<EvidenceStore>,
        project: &Path,
        retain_until_ms: u64,
    ) -> Result<Self, String> {
        if !matches!(
            plan.captured_subject()
                .snapshot
                .as_git()
                .map(|subject| subject.kind),
            Some(SubjectKind::Worktree)
        ) {
            return Err(
                "non-worktree candidate requires verified private materialization before execution"
                    .into(),
            );
        }
        Self::build(plan, contract, store, project, retain_until_ms, None)
    }
    /// v2显式选择私有scratch父目录；实际物化仅在Started并取得执行槽位后发生。
    /// 构造仍为同步只读准备，执行时才在scratch中创建/清理私有候选目录。
    pub fn new_v2(
        plan: &RegisteredPlan,
        contract: &CompiledContract,
        store: Arc<EvidenceStore>,
        project: &Path,
        retain_until_ms: u64,
        scratch_parent: &Path,
    ) -> Result<Self, String> {
        if !scratch_parent.is_absolute() || !scratch_parent.is_dir() {
            return Err("absolute existing private scratch parent required".into());
        }
        Self::build(
            plan,
            contract,
            store,
            project,
            retain_until_ms,
            Some(scratch_parent.to_path_buf()),
        )
    }
    fn build(
        plan: &RegisteredPlan,
        contract: &CompiledContract,
        store: Arc<EvidenceStore>,
        project: &Path,
        retain_until_ms: u64,
        scratch_parent: Option<PathBuf>,
    ) -> Result<Self, String> {
        if plan.captured_subject().snapshot.as_git().is_none() {
            return Err("release artifact execution provider is not implemented".into());
        }
        plan.assert_current(&store, project, now()?)?;
        if &plan.plan().verification().contract_digest != contract.digest()
            || plan.capture_binding().verifier_digest != guard_plan::verifier_digest(contract)?
        {
            return Err("compiled contract or verifier set differs from registered plan".into());
        }
        Ok(Self {
            plan: plan.clone(),
            contract: contract.clone(),
            store,
            slots: Arc::new(Semaphore::new(
                plan.plan().verification().budget.max_concurrency as usize,
            )),
            project: project.canonicalize().map_err(|e| e.to_string())?,
            retain_until_ms,
            runner: ProcessRunner::new(plan.plan().verification().budget.clone())?,
            scratch_parent,
        })
    }
    async fn bounded<T: Send + 'static>(
        &self,
        cancel: Option<&Cancellation>,
        deadline: std::time::Instant,
        action: impl FnOnce() -> Result<T, String> + Send + 'static,
    ) -> Result<T, CheckFailure> {
        static CHECKS: OnceLock<Arc<Semaphore>> = OnceLock::new();
        let checks = CHECKS.get_or_init(|| Arc::new(Semaphore::new(16))).clone();
        let deadline = tokio::time::Instant::from_std(
            deadline.min(std::time::Instant::now() + Duration::from_secs(30)),
        );
        let signal = cancel.cloned().unwrap_or_default();
        let permit = tokio::select! {
            result=tokio::time::timeout_at(deadline,checks.acquire_owned())=>result.map_err(|_|CheckFailure::TimedOut)?.map_err(|e|CheckFailure::Rejected(e.to_string()))?,
            _=cancelled(&signal)=>return Err(CheckFailure::Cancelled),
        };
        // 实际后台操作持有检查令牌，取消不会提前释放仍在使用的资源。
        let task = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            action()
        });
        tokio::select! {
            result=tokio::time::timeout_at(deadline,task)=>result.map_err(|_|CheckFailure::TimedOut)?.map_err(|e|CheckFailure::Rejected(e.to_string()))?.map_err(CheckFailure::Rejected),
            _=cancelled(&signal)=>Err(CheckFailure::Cancelled),
        }
    }
    async fn check_current(
        &self,
        cancel: Option<&Cancellation>,
        deadline: std::time::Instant,
    ) -> Result<(), CheckFailure> {
        self.check_execution(None, cancel, deadline).await
    }
    async fn check_execution(
        &self,
        workspace: Option<Arc<MaterializedSubject>>,
        cancel: Option<&Cancellation>,
        deadline: std::time::Instant,
    ) -> Result<(), CheckFailure> {
        let plan = self.plan.clone();
        let store = self.store.clone();
        let project = self.project.clone();
        self.bounded(cancel, deadline, move || {
            let source = plan.assert_current(&store, &project, now()?);
            let inputs = workspace.map_or(Ok(()), |workspace| workspace.verify_inputs());
            match (source, inputs) {
                (Ok(()), Ok(())) => Ok(()),
                (Err(source), Ok(())) => Err(format!("source-current: {source}")),
                (Ok(()), Err(inputs)) => Err(format!("materialized inputs: {inputs}")),
                (Err(source), Err(inputs)) => Err(format!(
                    "source-current: {source}; materialized inputs: {inputs}"
                )),
            }
        })
        .await
    }
    async fn materialize(
        &self,
        cancel: &Cancellation,
        deadline: std::time::Instant,
    ) -> Result<Arc<MaterializedSubject>, CheckFailure> {
        let subject = self.plan.captured_subject().clone();
        let project = self.project.clone();
        let scratch = self
            .scratch_parent
            .clone()
            .ok_or_else(|| CheckFailure::Rejected("v2 scratch not selected".into()))?;
        self.bounded(Some(cancel), deadline, move || {
            let workspace = guard_subject::materialize(
                &project,
                &subject.binding,
                &subject.selection,
                &subject,
                &scratch,
            )?;
            let parent = workspace
                .path()
                .parent()
                .ok_or("missing private materialization parent")?;
            let actual_scratch = parent
                .parent()
                .ok_or("missing scratch layout")?
                .canonicalize()
                .map_err(|e| e.to_string())?;
            if workspace.path().file_name() != Some(std::ffi::OsStr::new("inputs"))
                || actual_scratch != scratch.canonicalize().map_err(|e| e.to_string())?
            {
                return Err("materialization must use direct private TempDir/inputs under configured scratch".into());
            }
            if workspace.captured() != &subject {
                return Err("materialized subject differs from registered plan".into());
            }
            Ok(Arc::new(workspace))
        })
        .await
    }
    /// 只调度精确义务和编译pin；运行票、原始双流及终结metadata均持久化。
    /// future中止时runner清树，但本方法没有async Drop写库，running须显式恢复。
    pub async fn execute(
        &self,
        obligation_id: &Identifier,
        operation_id: &str,
        native: NativeVerifierCommand,
        cancel: Cancellation,
    ) -> Result<DispatchOutcome, String> {
        if self.scratch_parent.is_some() {
            return Err("v2 runtime requires explicit execute_v2 command profile".into());
        }
        let approved = NativeVerifierCommand::configuration_digest(&native.command)?;
        self.execute_native(obligation_id, operation_id, native, approved, false, cancel)
            .await
    }
    /// 执行已批准逻辑根模板，四种Git候选都先物化且只映射显式subject_root占位。
    pub async fn execute_v2(
        &self,
        obligation_id: &Identifier,
        operation_id: &str,
        native: NativeVerifierCommandV2,
        cancel: Cancellation,
    ) -> Result<DispatchOutcome, String> {
        if self.scratch_parent.is_none() {
            return Err("v1 runtime cannot consume v2 profile".into());
        }
        let approved = NativeVerifierCommandV2::configuration_digest(&native.command)?;
        self.execute_native(
            obligation_id,
            operation_id,
            NativeVerifierCommand {
                verifier_id: native.verifier_id,
                command: native.command,
            },
            approved,
            true,
            cancel,
        )
        .await
    }
    async fn execute_native(
        &self,
        obligation_id: &Identifier,
        operation_id: &str,
        native: NativeVerifierCommand,
        approved: Digest,
        v2: bool,
        cancel: Cancellation,
    ) -> Result<DispatchOutcome, String> {
        self.check_current(
            Some(&cancel),
            std::time::Instant::now() + Duration::from_secs(30),
        )
        .await
        .map_err(|e| format!("initial candidate check: {e:?}"))?;
        let verification = self.plan.plan().verification();
        let obligation = verification
            .obligations
            .iter()
            .find(|o| &o.obligation_id == obligation_id)
            .ok_or("unknown frozen obligation")?;
        let pin = self
            .contract
            .verifiers()
            .iter()
            .find(|p| p.verifier_id == obligation.verifier_id)
            .ok_or("missing compiled verifier pin")?;
        if native.verifier_id != obligation.verifier_id
            || native.command.executable_digest != pin.artifact_digest
            || approved != pin.config_digest
            || (if v2 {
                NativeVerifierCommandV2::environment_digest(&native.command.environment)?
            } else {
                NativeVerifierCommand::environment_digest(&native.command.environment)?
            }) != self.plan.capture_binding().environment_digest
            || native
                .command
                .directory
                .canonicalize()
                .map_err(|e| e.to_string())?
                != self.project
        {
            return Err(
                "native verifier, executable/config/environment pin or directory mismatch".into(),
            );
        }
        let identity = RunIdentity {
            binding_digest: digest(
                &json!({"taskBinding":self.plan.binding(),"planDigest":self.plan.plan().digest(),"activeHead":self.plan.head()}),
            )?,
            subject_digest: verification.subject_digest.clone(),
            contract_digest: verification.contract_digest.clone(),
            policy_digest: verification.policy_digest.clone(),
            verifier_digest: guard_plan::verifier_digest(&self.contract)?,
            environment_digest: self.plan.capture_binding().environment_digest.clone(),
        };
        let scope = RunScope {
            repository_id: self.contract.context().repository_id.clone(),
            task_id: self.plan.binding().task_id.clone(),
            obligation_id: obligation_id.clone(),
            approved_scope_digest: digest(&self.plan.binding().scope)?,
        };
        let ledger = RunLedger::new(&self.store);
        let ticket = match ledger.start(
            &scope,
            &identity,
            operation_id,
            self.retain_until_ms,
            now()?,
        )? {
            StartOutcome::Existing(ticket) => {
                return Ok(DispatchOutcome::Existing {
                    sequence: ticket.sequence().clone(),
                });
            }
            StartOutcome::Started(ticket) => ticket,
        };
        // start后再检查，避免排队/CAS期间已知漂移仍派发；失败留下明确incomplete终结。
        let deadline =
            std::time::Instant::now() + Duration::from_millis(verification.budget.timeout_ms);
        let slot = tokio::select! {
            value = tokio::time::timeout_at(tokio::time::Instant::from_std(deadline), self.slots.clone().acquire_owned()) => value.ok().and_then(Result::ok),
            _ = cancelled(&cancel) => None,
        };
        let mut workspace = None;
        let checked = if slot.is_some() {
            if v2 {
                match self.materialize(&cancel, deadline).await {
                    Ok(materialized) => {
                        workspace = Some(materialized);
                        self.check_execution(workspace.clone(), Some(&cancel), deadline)
                            .await
                    }
                    Err(error) => Err(error),
                }
            } else {
                self.check_current(Some(&cancel), deadline).await
            }
        } else if cancel.is_cancelled() {
            Err(CheckFailure::Cancelled)
        } else {
            Err(CheckFailure::TimedOut)
        };
        let mut invocation = InvocationReceipt {
            profile: if v2 {
                "guard.native-command/v2"
            } else {
                "guard.native-command/v1"
            },
            approved_config_digest: approved,
            subject_digest: verification.subject_digest.clone(),
            effective_invocation_digest: None,
            actual_environment_digest: None,
            working_directory: None,
            materialization: None,
        };
        let effective = if let Some(workspace) = &workspace {
            NativeVerifierCommandV2::render(&native.command, workspace.path())?
        } else {
            native.command
        };
        if !v2 || workspace.is_some() {
            let cwd = effective
                .directory
                .to_str()
                .ok_or("non-Unicode execution directory")?
                .to_owned();
            invocation.actual_environment_digest = Some(digest(&effective.environment)?);
            invocation.effective_invocation_digest = Some(digest(
                &json!({"schemaVersion":"guard.effective-invocation/v1","executable":effective.executable,
                "artifactDigest":effective.executable_digest,"argv":effective.args,"workingDirectory":cwd,"environment":effective.environment}),
            )?);
            invocation.working_directory = Some(cwd.clone());
            if let Some(workspace) = &workspace {
                let actual = workspace.captured();
                if actual != self.plan.captured_subject() {
                    return Err("materialized subject differs from registered plan".into());
                }
                let manifest = actual
                    .snapshot
                    .as_git()
                    .ok_or("Git materialization required")?
                    .input_manifest_digest
                    .clone();
                invocation.materialization = Some(MaterializationReceipt {
                    subject_digest: actual.identity_digest.clone(),
                    input_manifest_digest: manifest.clone(),
                    receipt_digest: digest(
                        &json!({"schemaVersion":"guard.materialization-receipt/v1","subjectDigest":actual.identity_digest,"inputManifestDigest":manifest,"workingDirectory":cwd}),
                    )?,
                });
            }
        }
        let result = match &checked {
            Ok(()) => self.runner.run_until(effective, cancel, deadline).await,
            Err(CheckFailure::Rejected(_)) => Err("input recheck rejected before dispatch".into()),
            Err(error) => Ok(guard_runner::RunOutput {
                status: if matches!(error, CheckFailure::Cancelled) {
                    ExecutionStatus::Cancelled
                } else {
                    ExecutionStatus::TimedOut
                },
                reason: if matches!(error, CheckFailure::Cancelled) {
                    StopReason::Cancelled
                } else {
                    StopReason::Timeout
                },
                exit_code: None,
                stdout: Vec::new(),
                stderr: Vec::new(),
                output_truncated: false,
                process_id: None,
                started_at: None,
                finished_at: None,
            }),
        };
        let after = self
            .check_execution(
                workspace.clone(),
                None,
                std::time::Instant::now() + Duration::from_secs(30),
            )
            .await;
        let drifted = matches!(&checked, Err(CheckFailure::Rejected(_)))
            || matches!(&after, Err(CheckFailure::Rejected(_)));
        let recheck_error =
            checked
                .as_ref()
                .err()
                .or(after.as_ref().err())
                .map(|error| match error {
                    CheckFailure::Rejected(message) => format!(
                        "candidate rejected: {}",
                        message.chars().take(1024).collect::<String>()
                    ),
                    CheckFailure::TimedOut => "candidate check timed out".into(),
                    CheckFailure::Cancelled => "candidate check cancelled".into(),
                });
        let (status, exit, reason, stdout, stderr, truncated, started_at, finished_at) =
            match result {
                Ok(output) => (
                    output.status,
                    output.exit_code,
                    format!("{:?}", output.reason),
                    output.stdout,
                    output.stderr,
                    output.output_truncated,
                    output.started_at,
                    output.finished_at,
                ),
                Err(_) if matches!(&checked, Err(CheckFailure::Rejected(_))) => (
                    ExecutionStatus::NotRun,
                    None,
                    "InputRejected".into(),
                    Vec::new(),
                    Vec::new(),
                    false,
                    None,
                    None,
                ),
                Err(_) => (
                    ExecutionStatus::Failed,
                    None,
                    "DispatchError".into(),
                    Vec::new(),
                    Vec::new(),
                    false,
                    None,
                    None,
                ),
            };
        let state = if drifted || truncated || after.is_err() {
            RunState::Incomplete
        } else if reason == format!("{:?}", StopReason::Cancelled) {
            RunState::Cancelled
        } else if reason == format!("{:?}", StopReason::Timeout) {
            RunState::Timeout
        } else if reason == "DispatchError" {
            RunState::Error
        } else if status == ExecutionStatus::Completed && exit == Some(0) {
            RunState::Incomplete
        } else {
            RunState::Fail
        };
        let stamp = now()?;
        let save_stream = |label: &str, bytes: &[u8]| -> Result<ArtifactRef, String> {
            let key = digest(
                &json!({"identity":identity,"scope":scope,"sequence":ticket.sequence(),"operation":operation_id,"stream":label}),
            )?;
            let operation = format!("runtime:{label}:{}", key.as_str());
            let summary = self
                .store
                .put(&operation, bytes, self.retain_until_ms, stamp)?;
            Ok(ArtifactRef {
                operation,
                digest: summary.content_digest,
                size: bytes.len() as u64,
            })
        };
        let mut observation = ExecutionObservation {
            schema_version: "guard.execution-observation/v1",
            obligation_id: obligation_id.clone(),
            verifier_id: pin.verifier_id.clone(),
            identity: identity.clone(),
            plan_digest: self.plan.plan().digest().clone(),
            invocation,
            sequence: ticket.sequence().clone(),
            execution_status: status,
            exit_code: exit,
            started_at,
            finished_at,
            stop_reason: reason,
            output_truncated: truncated,
            input_drifted: drifted,
            recheck_error,
            state,
            stdout: save_stream("stdout", &stdout)?,
            stderr: save_stream("stderr", &stderr)?,
            trust: "local_unverified",
            finalization: None,
        };
        let metadata = guard_protocol::canonicalize(
            &serde_json::to_vec(&observation).map_err(|e| e.to_string())?,
        )?;
        observation.finalization = Some(ledger.finalize(&ticket, state, &metadata, stamp)?);
        Ok(DispatchOutcome::Finished(Box::new(observation)))
    }
}
