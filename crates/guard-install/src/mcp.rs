use crate::{Options, Tool, cli};
use serde_json::{Value, json};
use std::io::{BufRead, Read, Write};

pub(crate) fn serve(tool: &Tool, options: &Options) -> Result<(), String> {
    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    let stdout = std::io::stdout();
    let mut writer = stdout.lock();
    let mut initialized = false;
    let mut ready = false;
    loop {
        let mut line = Vec::new();
        // 有界 JSON 行，防止输入耗尽内存。超限关闭会话，不在 stdout 混入日志。
        let size = std::io::Read::by_ref(&mut reader)
            .take(1024 * 1024 + 1)
            .read_until(b'\n', &mut line)
            .map_err(|e| e.to_string())?;
        if size == 0 {
            break;
        }
        if size > 1024 * 1024 {
            return Err("MCP input exceeds limit".into());
        }
        let response = match serde_json::from_slice::<Value>(&line) {
            Ok(request) => respond(tool, options, &request, &mut initialized, &mut ready),
            Err(_) => Some(
                json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":"Parse error"}}),
            ),
        };
        if let Some(value) = response {
            writeln!(writer, "{value}").map_err(|e| e.to_string())?;
            writer.flush().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn respond(
    tool: &Tool,
    options: &Options,
    request: &Value,
    initialized: &mut bool,
    ready: &mut bool,
) -> Option<Value> {
    if request.get("id").is_none() && request["method"].is_string() {
        if request["jsonrpc"] == "2.0"
            && request["method"] == "notifications/initialized"
            && *initialized
        {
            *ready = true;
        }
        return None;
    }
    let id = request["id"].clone();
    let error = |code, message: &str| json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}});
    if request["jsonrpc"] != "2.0"
        || !request["method"].is_string()
        || !(request["id"].is_string() || request["id"].is_number() || request["id"].is_null())
    {
        return Some(error(-32600, "Invalid request"));
    }
    if request["method"] != "initialize" && request["method"] != "ping" && !*ready {
        return Some(error(-32000, "Initialize session before using tools"));
    }
    if request["method"] == "initialize" {
        if *initialized {
            return Some(error(-32600, "Already initialized"));
        }
        if !request["params"]["protocolVersion"].is_string()
            || !request["params"]["capabilities"].is_object()
            || !request["params"]["clientInfo"]["name"].is_string()
            || !request["params"]["clientInfo"]["version"].is_string()
        {
            return Some(error(-32602, "Invalid initialization parameters"));
        }
        *initialized = true;
    }
    let context = format!("{}_context", tool.name);
    let instructions = format!("{}_instructions", tool.name);
    let result = match request["method"].as_str() {
        Some("initialize") => {
            json!({"protocolVersion":"2024-11-05","capabilities":{"tools":{"listChanged":false}},"serverInfo":{"name":tool.name,"version":tool.version},"instructions":"Inspect tools/list for implemented context and domain capabilities. No approval is granted by this server."})
        }
        Some("ping") => json!({}),
        Some("tools/list") => {
            let mut tools = [context.clone(),instructions.clone()].iter().map(|name|json!({"name":name,"description":"Read explicit project context, native specification references or instructions without initializing or editing files. Specification ready is not approval.","inputSchema":{"type":"object","properties":{"project":{"type":"string"},"spec_system":{"type":"string","enum":["openspec","speckit","superpowers"]},"spec_target":{"type":"string"}},"required":["project"],"additionalProperties":false}})).collect::<Vec<_>>();
            tools.extend(tool.mcp_tools.iter().map(|entry|json!({"name":entry.name,"description":entry.description,"inputSchema":(entry.schema)()})));
            json!({"tools":tools})
        }
        Some("tools/call") => {
            let args = &request["params"]["arguments"];
            if let Some(entry) = tool
                .mcp_tools
                .iter()
                .find(|entry| request["params"]["name"] == entry.name)
            {
                let value = match (entry.call)(args) {
                    Ok(value) => {
                        json!({"content":[{"type":"text","text":value.to_string()}],"isError":false})
                    }
                    Err(message) => {
                        json!({"content":[{"type":"text","text":message}],"isError":true})
                    }
                };
                return Some(json!({"jsonrpc":"2.0","id":id,"result":value}));
            }
            if !args.is_object()
                || args.as_object().is_some_and(|a| {
                    a.keys().any(|key| {
                        !["project", "spec_system", "spec_target"].contains(&key.as_str())
                    })
                })
                || !args["project"].is_string()
            {
                return Some(error(-32602, "Explicit project string required"));
            }
            if !std::path::Path::new(args["project"].as_str().unwrap()).is_absolute() {
                return Some(error(-32602, "Project must be absolute"));
            }
            let mut command = match request["params"]["name"].as_str() {
                Some(name) if name == context => vec!["context".into()],
                Some(name) if name == instructions => vec!["instructions".into(), "render".into()],
                _ => return Some(error(-32602, "Unknown tool")),
            };
            command.extend(["--project".into(), args["project"].as_str().unwrap().into()]);
            for (key, flag) in [
                ("spec_system", "--spec-system"),
                ("spec_target", "--spec-target"),
            ] {
                if let Some(value) = args.get(key) {
                    let Some(value) = value.as_str().filter(|s| !s.is_empty()) else {
                        return Some(error(
                            -32602,
                            "Specification selection must be nonempty strings",
                        ));
                    };
                    command.extend([flag.into(), value.into()]);
                }
            }
            for (flag, path) in [
                ("--gitflow-root", &options.gitflow_root),
                ("--python", &options.python),
            ] {
                if let Some(path) = path {
                    command.extend([flag.to_owned(), path.to_string_lossy().into_owned()]);
                }
            }
            match cli::run(tool, &command) {
                Ok(value) => {
                    json!({"content":[{"type":"text","text":value.to_string()}],"isError":false})
                }
                Err(message) => json!({"content":[{"type":"text","text":message}],"isError":true}),
            }
        }
        _ => return Some(error(-32601, "Method not found")),
    };
    Some(json!({"jsonrpc":"2.0","id":id,"result":result}))
}
