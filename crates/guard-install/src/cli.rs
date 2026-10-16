use crate::{Options, Tool, instructions, integration, mcp};
use serde_json::{Value, json};
use std::process::ExitCode;

/// 处理共享接入命令；返回 None 时交回原有专业 CLI，不改变其退出码。
pub fn dispatch(tool: &Tool, args: &[String]) -> Option<ExitCode> {
    let command = args.first()?.as_str();
    let recognized = matches!(
        command,
        "install" | "uninstall" | "context" | "instructions" | "spec"
    ) || command == "doctor" && args.iter().any(|a| a == "--integration")
        || command == "serve" && args.get(1).is_some_and(|a| a == "--mcp");
    if !recognized {
        return None;
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "{} spec context --project PATH [--spec-system openspec|speckit|superpowers] [--spec-target RELATIVE_PATH]",
            tool.name
        );
        println!(
            "{} install --agents codex,claude,gemini|none [--scope project|user] [--project PATH] [--home PATH] [--bin-dir PATH] [--dry-run]\n{} uninstall [--remove-binary] [--dry-run]\n{} context | instructions render|sync | doctor --integration\n{} serve --mcp\nGitGuard adapter: --gitflow-root PATH --python EXECUTABLE",
            tool.name, tool.name, tool.name, tool.name
        );
        return Some(ExitCode::SUCCESS);
    }
    if command == "serve" {
        let result = Options::parse(&args[2..]).and_then(|options| mcp::serve(tool, &options));
        return Some(match result {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                ExitCode::from(4)
            }
        });
    }
    let result = run(tool, args);
    Some(match result {
        Ok(value) => {
            if !value.is_null() {
                println!("{}", serde_json::to_string_pretty(&value).unwrap());
            }
            if value["status"] == "incomplete" {
                ExitCode::from(3)
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            println!(
                "{}",
                json!({"schema_version":1,"tool":tool.name,"status":"error","message":error,"quality_decision":"not_evaluated"})
            );
            ExitCode::from(4)
        }
    })
}

pub(crate) fn run(tool: &Tool, args: &[String]) -> Result<Value, String> {
    let op = args.first().map(String::as_str).ok_or("missing command")?;
    let offset = if op == "instructions" || op == "spec" {
        2
    } else {
        1
    };
    let options = Options::parse(
        args.get(offset..)
            .ok_or("instructions requires render or sync")?,
    )?;
    match op {
        "install" => integration::install(tool, &options, false),
        "uninstall" => integration::uninstall(tool, &options),
        "doctor" => integration::doctor(tool, &options),
        "context" => instructions::context(tool, &options),
        "spec" if args.get(1).map(String::as_str) == Some("context") => {
            crate::specification_context(
                &options.project,
                options.spec_system.as_deref(),
                options.spec_target.as_deref(),
            )
        }
        "instructions" => match args.get(1).map(String::as_str) {
            Some("render") => {
                Ok(json!({"tool":tool.name,"instructions":instructions::render(tool,&options)?}))
            }
            Some("sync") => integration::install(tool, &options, true),
            _ => Err("instructions requires render or sync".into()),
        },
        _ => Err("unknown integration command".into()),
    }
}
