use crate::cli_failure::CliFailure;
use crate::{
    host_config::HostConfig,
    prepared::{self, Prepared},
    safe_files,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use clap::ArgMatches;
use serde_json::{Value, json};
use std::path::Path;
fn argument<'a>(args: &'a ArgMatches, key: &str) -> Result<&'a str, String> {
    args.get_one::<String>(key)
        .map(String::as_str)
        .ok_or_else(|| format!("missing {key}"))
}
/// 接通六组真实内核操作；不创建批准或质量结论。无Java对应类型。
pub(crate) async fn execute(
    args: &ArgMatches,
    host: Result<HostConfig, String>,
) -> Result<(u8, Value), CliFailure> {
    let (group, sub) = args.subcommand().ok_or("command required")?;
    if group == "contract" && sub.subcommand_name() == Some("validate") {
        let child = sub
            .subcommand_matches("validate")
            .ok_or("validate arguments required")?;
        let bytes = safe_files::read(Path::new(argument(child, "file")?), 1024 * 1024)
            .map_err(CliFailure::Usage)?;
        let issues = guard_contract::SchemaValidator::protocol()?
            .validate_bytes(&bytes)
            .map_err(CliFailure::Usage)?;
        if !issues.is_empty() {
            return Ok((1, json!({"status":"rejected","issues":issues})));
        }
        let document = guard_protocol::decode(&bytes).map_err(CliFailure::Rejected)?;
        if !matches!(
            document.document(),
            guard_protocol::ProtocolDocument::ContractSnapshot(_)
        ) {
            return Ok((
                1,
                json!({"status":"rejected","reason":"ContractSnapshot required"}),
            ));
        }
        return Ok((
            0,
            json!({"status":"completed","validation":"valid_candidate","approval":"none","sideEffects":[]}),
        ));
    }
    let host = host.and_then(|host| host.reload_same_profile());
    if group == "doctor" {
        if let Err(reason) = &host {
            return Ok((
                3,
                json!({"status":"incomplete","diagnostics":[reason],"discovery":crate::discovery::unavailable(reason),"authority":"not_inspected","verifiers":[],"sideEffects":[]}),
            ));
        }
    }
    let host = host?;
    if group == "doctor" {
        let (_, probes, report) = prepared::probe(&host).await?;
        let complete = probes.0.values().all(Result::is_ok);
        // 配置读取不等于真实批准：doctor不伪装完整执行或签名资格。
        return Ok((
            if complete { 0 } else { 3 },
            json!({"status":if complete{"completed"}else{"incomplete"},"discovery":crate::discovery::observe(&host)?,"verifiers":report,"authority":"configured_not_yet_validated","sideEffects":["private_probe_directory","approved_version_probe_processes"]}),
        ));
    }
    if group == "evidence" && sub.subcommand_name() != Some("source-verify") {
        let (action, child) = sub.subcommand().ok_or("evidence action required")?;
        let store = prepared::store(&host, false)?;
        let operation = argument(child, "operation")?;
        // open会进行崩溃恢复/隔离，必须披露，不能把它写成纯文件读取。
        let raw = store
            .read_private(operation, prepared::now()?)
            .map_err(CliFailure::Internal)?;
        let summary = store.summary(operation).map_err(CliFailure::Internal)?;
        let mut result = json!({"status":"completed","integrity":"valid","trust":"local_unverified","summary":summary,"sideEffects":["private_store_recovery"]});
        if action == "show" && child.get_flag("raw") {
            result["privateRaw"] = json!({"encoding":"base64","bytes":STANDARD.encode(raw)});
        }
        return Ok((0, result));
    }
    if group == "policy" && sub.subcommand_name() == Some("validate") {
        let package = prepared::package(&host)?;
        return Ok((
            0,
            json!({"status":"completed","packageDigest":package.digest(),"manifest":package.manifest(),"approval":"none","sideEffects":[]}),
        ));
    }
    let leaf = if group == "plan" || group == "policy" || group == "evidence" {
        sub.subcommand().map(|(_, v)| v).unwrap_or(sub)
    } else {
        sub
    };
    if let Ok(Some(file)) = leaf.try_get_one::<String>("selection") {
        let bytes = safe_files::read(Path::new(file), 1024 * 1024).map_err(CliFailure::Usage)?;
        safe_files::decode::<guard_subject::Selection>(&bytes).map_err(CliFailure::Usage)?;
    }
    let prepared = Prepared::compile(host).await?;
    if group == "contract" {
        return Ok((
            0,
            json!({"status":"completed","contractDigest":prepared.contract.digest(),"rules":prepared.contract.rules(),"obligations":prepared.contract.obligations(),"approval":"independently_verified_receipts","sideEffects":["approved_version_probes_private_cwd"]}),
        ));
    }
    if group == "plan" {
        let (action, child) = sub.subcommand().ok_or("plan action required")?;
        let selection = argument(child, "selection")?;
        if action == "restore" {
            let store = prepared.store(false)?;
            let plan = prepared.restore(selection, argument(child, "operation")?, &store)?;
            return Ok((
                0,
                json!({"status":"completed","plan":plan.plan().verification(),"planDigest":plan.plan().digest(),"head":plan.head(),"sideEffects":["private_store_recovery","approved_version_probes_private_cwd"]}),
            ));
        }
        let outcome = prepared.freeze(selection)?;
        let Some(plan) = outcome.plan() else {
            return Ok((
                3,
                json!({"status":"incomplete","issues":outcome.issues(),"obligations":outcome.contract_obligations()}),
            ));
        };
        let mut report = json!({"status":"completed","plan":plan.verification(),"planDigest":plan.digest(),"sideEffects":["private_git_capture","approved_version_probes_private_cwd"]});
        if action == "register" {
            let store = prepared.store(true)?;
            let until = prepared.retain()?;
            outcome.persist(
                &store,
                argument(child, "operation")?,
                until,
                prepared::now()?,
            )?;
            let registered = outcome.register(&store, until, prepared::now()?)?;
            report["head"] = json!(registered.head());
            report["sideEffects"] = json!([
                "private_store_write",
                "private_git_capture",
                "approved_version_probes_private_cwd"
            ]);
        }
        return Ok((0, report));
    }
    let child = if group == "policy" {
        sub.subcommand_matches("evaluate")
            .ok_or("policy evaluate required")?
    } else if group == "evidence" {
        sub.subcommand_matches("source-verify")
            .ok_or("source verify required")?
    } else {
        sub
    };
    let store = prepared.store(false)?;
    let plan = prepared.restore(
        argument(child, "selection")?,
        argument(child, "plan-operation")?,
        &store,
    )?;
    if group == "evidence" {
        let ports = std::sync::Arc::new(crate::file_ports::FilePorts(prepared.host.clone()));
        let verifier = guard_authority::ProducerVerifier::new(ports.clone(), ports.clone(), ports);
        let raw = safe_files::read(Path::new(argument(child, "file")?), 1024 * 1024)
            .map_err(CliFailure::Usage)?;
        let project = prepared.host.project()?;
        let context = guard_authority::ProducerRestoreContext {
            plan: &plan,
            contract: &prepared.contract,
            store: &store,
            project: &project,
        };
        let evidence = verifier
            .verify_restored(&raw, &context, &prepared.host.field("audience")?)
            .map_err(CliFailure::Rejected)?;
        let attestation = evidence.attestation();
        return Ok((
            0,
            json!({"status":"completed","source":"independently_verified_current","attestationId":attestation.attestation_id,"producerId":attestation.provenance.producer.producer_id,"sequence":attestation.provenance.sequence,"rawDigest":guard_protocol::digest_json(&raw)?,"quality":"not_evaluated","sideEffects":["private_store_recovery","approved_version_probes_private_cwd"]}),
        ));
    }
    let scratch = prepared.host.field::<std::path::PathBuf>("scratchRoot")?;
    let cancel = guard_runner::Cancellation::new();
    let monitor = crate::cancellation_monitor::CancellationMonitor::start(
        cancel.clone(),
        &prepared.host,
        argument(child, "operation")?,
        child.get_one::<String>("cancel-request"),
    )?;
    let result:Result<(u8,Value),CliFailure>=async{
  if group=="run"{
   let obligation=guard_protocol::Identifier::new(argument(child,"obligation")?)?;
   let required=plan.plan().verification().obligations.iter().find(|o|o.obligation_id==obligation).ok_or_else(||CliFailure::Rejected("obligation not in frozen plan".into()))?;
   let native=prepared.verifiers.iter().find(|v|v.verifier_id==required.verifier_id&&v.kind=="domain").ok_or("required native verifier missing")?;
   let runtime=guard_runtime::Runtime::new_v2(&plan,&prepared.contract,store,&prepared.host.project()?,prepared.retain()?,&scratch)?;
   let observed=runtime.execute_v2_with_receipt(&obligation,argument(child,"operation")?,guard_runtime::NativeVerifierCommandV2{verifier_id:native.verifier_id.clone(),command:native.command(&prepared.host.project()?,false)},cancel.clone()).await?;
   let qualification_error=observed.qualification_error().map(str::to_owned);
   let (outcome,completion)=observed.into_parts();
   match outcome{
    guard_runtime::DispatchOutcome::Existing{sequence}=>Ok((3,json!({"status":"incomplete","reason":"already_running_not_redispatched","sequence":sequence}))),
    guard_runtime::DispatchOutcome::Finished(observation)=>{let code=crate::run_exit::code(observation.execution_status,observation.exit_code,observation.input_drifted,observation.output_truncated,observation.recheck_error.as_deref(),observation.finalization,cancel.is_cancelled());Ok((code,json!({"status":if code==0{"completed"}else{"incomplete"},"observation":observation,"sourceCompletionAvailable":completion.is_some(),"qualificationError":qualification_error,"trust":"local_unverified","sideEffects":["private_candidate_materialization","pinned_native_process_tree","private_store_write"]})))},
   }
  }else{
   let opa=prepared.verifiers.iter().find(|v|v.kind=="policy_engine").ok_or("policy engine missing")?;
   let evaluator=guard_policy::BoundPolicyEvaluator::prepare(&prepared.host.project()?,&prepared.contract,&plan,prepared::opa_config(&prepared.host,opa)?,prepared.package.clone(),store,&scratch,prepared.retain()?)?;
   let facts=safe_files::read(Path::new(argument(child,"facts")?),1024*1024).map_err(CliFailure::Usage)?;
   safe_files::decode::<Value>(&facts).map_err(CliFailure::Usage)?;
   let receipt=evaluator.evaluate(&facts,argument(child,"operation")?,cancel.clone()).await?;
   let code=if cancel.is_cancelled(){130}else if receipt.evaluation.incomplete{3}else{match receipt.evaluation.decision.as_ref().map(|v|v.decision){Some(guard_protocol::Decision::Deny)=>1,Some(guard_protocol::Decision::ReviewRequired|guard_protocol::Decision::Incomplete|guard_protocol::Decision::NotEvaluated)=>3,Some(_)=>0,None=>3}};
   Ok((code,json!({"status":if code==0{"completed"}else if code==1{"rejected"}else{"incomplete"},"receipt":receipt,"trust":"local_unverified","sideEffects":["pinned_opa_process","private_policy_materialization","private_store_write"]})))
  }
 }.await;
    let failure = monitor.failure();
    drop(monitor);
    if let Some(error) = failure {
        match result {
            Ok((_, mut payload)) => {
                payload["status"] = json!("incomplete");
                payload["invalidCancellationRequest"] = json!(error);
                return Ok((2, payload));
            }
            Err(_) => return Err(CliFailure::Usage(error)),
        }
    }
    result
}
