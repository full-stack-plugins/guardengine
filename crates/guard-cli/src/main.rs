mod arguments;
mod bootstrap;
mod cancel_request;
mod cancellation_monitor;
mod cli_failure;
mod configured_verifier;
mod discovery;
mod file_ports;
mod host_config;
mod operations;
mod prepared;
mod probe_results;
mod run_exit;
mod safe_files;
use clap::error::ErrorKind;
use serde_json::json;
fn main() -> std::process::ExitCode {
    let args = match arguments::command().try_get_matches() {
        Ok(v) => v,
        Err(e) => {
            let code = if matches!(e.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
                0
            } else {
                2
            };
            let _ = e.print();
            return code.into();
        }
    };
    let host = host_config::HostConfig::load().and_then(|host| {
        bootstrap::configure(&host)?;
        Ok(host)
    });
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("guardcore runtime initialization failed: {e}");
            return 4.into();
        }
    };
    let (code, payload) = match runtime.block_on(operations::execute(&args, host)) {
        Ok(v) => v,
        Err(e) => (
            e.code(),
            json!({"status":if e.code()==1{"rejected"}else if e.code()==4{"error"}else{"incomplete"},"diagnostics":[e.message()]}),
        ),
    };
    println!(
        "{}",
        json!({"schemaVersion":"guard.cli-result/v1","command":args.subcommand_name(),"status":payload.get("status").cloned().unwrap_or(json!("completed")),"hostProtection":"unverified","mode":"cooperative","quality":"not_evaluated","grant":null,"result":payload})
    );
    code.into()
}
