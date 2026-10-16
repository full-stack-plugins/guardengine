use crate::{
    Options, Tool, files, instructions, mutation::Mutation, owned_entry::OwnedEntry,
    receipt::Receipt,
};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

fn receipt_path(tool: &Tool, options: &Options) -> PathBuf {
    options.state_dir().join(format!("{}.json", tool.name))
}

pub(crate) fn load(tool: &Tool, options: &Options) -> Result<Option<Receipt>, String> {
    let Some(bytes) = files::read(&receipt_path(tool, options), files::MAX_BYTES)? else {
        return Ok(None);
    };
    let receipt: Receipt =
        serde_json::from_slice(&bytes).map_err(|e| format!("invalid installation receipt: {e}"))?;
    if receipt.schema_version != 1 || receipt.tool != tool.name {
        return Err("installation receipt identity mismatch".into());
    }
    let allowed = options.targets(&receipt.agents);
    let mut seen = std::collections::HashSet::new();
    if receipt
        .entries
        .iter()
        .any(|e| !allowed.contains(&(e.path.clone(), e.kind.clone())) || !seen.insert(&e.path))
        || allowed.len() != receipt.entries.len()
    {
        return Err("receipt contains unexpected targets".into());
    }
    files::safe_path(&receipt.executable)?;
    if receipt.binary_digest.is_some()
        && receipt.executable.file_name().and_then(|s| s.to_str())
            != Some(&format!("{}{}", tool.name, std::env::consts::EXE_SUFFIX))
    {
        return Err("invalid owned binary path".into());
    }
    Ok(Some(receipt))
}

fn text(bytes: Option<&[u8]>) -> Result<String, String> {
    String::from_utf8(bytes.unwrap_or_default().to_vec()).map_err(|_| "target is not UTF-8".into())
}

fn bounds(tool: &Tool, content: &str) -> Result<Option<(usize, usize)>, String> {
    let start = format!("<!-- {}_START -->", tool.name.to_uppercase());
    let end = format!("<!-- {}_END -->", tool.name.to_uppercase());
    let starts: Vec<_> = content.match_indices(&start).collect();
    let ends: Vec<_> = content.match_indices(&end).collect();
    match (starts.as_slice(), ends.as_slice()) {
        ([], []) => Ok(None),
        ([(s, _)], [(e, _)]) if s < e => Ok(Some((*s, *e + end.len()))),
        _ => Err("duplicate, missing or reversed managed markers".into()),
    }
}

fn json_config(content: &str) -> Result<Value, String> {
    let value = if content.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(content).map_err(|e| format!("invalid MCP JSON: {e}"))?
    };
    if !value.is_object() || value.get("mcpServers").is_some_and(|v| !v.is_object()) {
        return Err("MCP JSON must contain object mcpServers".into());
    }
    Ok(value)
}

fn current(tool: &Tool, kind: &str, content: &str) -> Result<Option<String>, String> {
    match kind {
        "markdown" => Ok(bounds(tool, content)?.map(|(s, e)| content[s..e].to_owned())),
        "json" => Ok(json_config(content)?
            .get("mcpServers")
            .and_then(|v| v.get(tool.name))
            .map(Value::to_string)),
        "toml" => {
            let doc: toml_edit::DocumentMut = content
                .parse()
                .map_err(|e| format!("invalid MCP TOML: {e}"))?;
            if let Some(servers) = doc.get("mcp_servers") {
                if !servers.is_table() {
                    return Err("mcp_servers must be a TOML table".into());
                }
            }
            Ok(doc
                .get("mcp_servers")
                .and_then(|v| v.get(tool.name))
                .map(|v| v.to_string()))
        }
        _ => Err("unsupported owned file kind".into()),
    }
}

fn edit(
    tool: &Tool,
    kind: &str,
    content: &str,
    replacement: Option<&str>,
    separator: &str,
) -> Result<String, String> {
    match kind {
        "markdown" => match (bounds(tool, content)?, replacement) {
            (Some((s, e)), Some(block)) => Ok(format!("{}{block}{}", &content[..s], &content[e..])),
            (Some((s, e)), None) => {
                let start = if content[..s].ends_with(separator) {
                    s - separator.len()
                } else {
                    s
                };
                Ok(format!("{}{}", &content[..start], &content[e..]))
            }
            (None, Some(block)) => Ok(format!("{content}{separator}{block}")),
            (None, None) => Ok(content.into()),
        },
        "json" => {
            let mut doc = json_config(content)?;
            if let Some(value) = replacement {
                if doc.get("mcpServers").is_none() {
                    doc["mcpServers"] = json!({});
                }
                doc["mcpServers"][tool.name] =
                    serde_json::from_str(value).map_err(|e| format!("invalid MCP entry: {e}"))?;
            } else if let Some(servers) = doc.get_mut("mcpServers").and_then(Value::as_object_mut) {
                servers.remove(tool.name);
                if servers.is_empty() {
                    doc.as_object_mut().unwrap().remove("mcpServers");
                }
            }
            Ok(format!(
                "{}\n",
                serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())?
            ))
        }
        "toml" => {
            let mut doc: toml_edit::DocumentMut = content
                .parse()
                .map_err(|e| format!("invalid MCP TOML: {e}"))?;
            if let Some(value) = replacement {
                let entry: toml_edit::DocumentMut =
                    value.parse().map_err(|e| format!("invalid entry: {e}"))?;
                if doc.get("mcp_servers").is_none() {
                    doc["mcp_servers"] = toml_edit::Item::Table(toml_edit::Table::new());
                }
                doc["mcp_servers"][tool.name] = entry["server"].clone();
            } else if let Some(servers) = doc
                .get_mut("mcp_servers")
                .and_then(toml_edit::Item::as_table_mut)
            {
                servers.remove(tool.name);
                if servers.is_empty() {
                    doc.remove("mcp_servers");
                }
            }
            Ok(doc.to_string())
        }
        _ => Err("unknown file kind".into()),
    }
}

fn server_entry(executable: &Path, kind: &str, options: &Options) -> String {
    let command = executable.to_string_lossy();
    let mut arguments = vec!["serve".to_owned(), "--mcp".to_owned()];
    // 适配器只来自安装者显式参数，固化在宿主配置；不从被查询项目执行任意程序。
    for (flag, path) in [
        ("--gitflow-root", &options.gitflow_root),
        ("--python", &options.python),
    ] {
        if let Some(path) = path {
            arguments.extend([flag.to_owned(), path.to_string_lossy().into_owned()]);
        }
    }
    if kind == "json" {
        json!({"command":command,"args":arguments}).to_string()
    } else {
        let mut doc = toml_edit::DocumentMut::new();
        doc["server"]["command"] = toml_edit::value(command.as_ref());
        let mut args = toml_edit::Array::new();
        for argument in arguments {
            args.push(argument);
        }
        doc["server"]["args"] = toml_edit::value(args);
        doc.to_string()
    }
}

fn owned_content(
    tool: &Tool,
    prior: Option<&OwnedEntry>,
    actual: Option<&str>,
) -> Result<(), String> {
    match (prior, actual) {
        (None, None) => Ok(()),
        (Some(entry), Some(value)) if entry.content == value => Ok(()),
        _ => Err(format!(
            "{} owned entry conflict; preserve edits and reconcile before retry",
            tool.name
        )),
    }
}

pub(crate) fn install(tool: &Tool, options: &Options, sync: bool) -> Result<Value, String> {
    let old = load(tool, options)?;
    if sync && old.is_none() {
        return Err("not installed in this scope".into());
    }
    let agents = options
        .agents
        .clone()
        .or_else(|| old.as_ref().map(|r| r.agents.clone()))
        .ok_or("select --agents codex,claude,gemini or none")?;
    if let Some(prior) = &old {
        if !prior.agents.is_empty() && prior.agents != agents {
            return Err(
                "agent selection changed; uninstall this integration before changing targets"
                    .into(),
            );
        }
    }
    let source = std::env::current_exe().map_err(|e| e.to_string())?;
    let executable = options
        .bin_dir
        .as_ref()
        .map(|d| d.join(format!("{}{}", tool.name, std::env::consts::EXE_SUFFIX)))
        .or_else(|| old.as_ref().map(|r| r.executable.clone()))
        .unwrap_or_else(|| source.clone());
    if options.bin_dir.is_none() && executable.canonicalize().ok() != source.canonicalize().ok() {
        return Err(
            "repeat explicit --bin-dir to update an installation owned by another executable"
                .into(),
        );
    }
    if old
        .as_ref()
        .is_some_and(|r| r.binary_digest.is_some() && r.executable != executable)
    {
        return Err("owned bin-dir changed; uninstall --remove-binary before relocating".into());
    }
    let mut changes = vec![];
    let mut binary_digest = old.as_ref().and_then(|r| r.binary_digest.clone());
    if options.bin_dir.is_some() || binary_digest.is_some() {
        let source_bytes =
            files::read(&source, 512 * 1024 * 1024)?.ok_or("running executable unavailable")?;
        let before = files::read(&executable, 512 * 1024 * 1024)?;
        if let Some(bytes) = &before {
            if old.as_ref().and_then(|r| r.binary_digest.as_ref()) != Some(&files::digest(bytes)) {
                return Err(
                    "binary target exists without matching ownership; choose another bin-dir"
                        .into(),
                );
            }
        }
        binary_digest = Some(files::digest(&source_bytes));
        changes.push(Mutation {
            path: executable.clone(),
            before,
            after: Some(source_bytes),
            executable: true,
        });
    }
    let effective = options.clone();
    if let Some(prior) = &old {
        if (prior.gitflow_root.is_some() && effective.gitflow_root.is_none())
            || (prior.python.is_some() && effective.python.is_none())
        {
            return Err("Repeat trusted --gitflow-root and --python when refreshing this installation; project receipts never authorize execution".into());
        }
    }
    let block = instructions::render(tool, &effective)?;
    let mut entries = vec![];
    for (path, kind) in options.targets(&agents) {
        let before = files::read(&path, files::MAX_BYTES)?;
        let original = text(before.as_deref())?;
        let prior = old
            .as_ref()
            .and_then(|r| r.entries.iter().find(|e| e.path == path));
        owned_content(tool, prior, current(tool, &kind, &original)?.as_deref())?;
        let separator = prior.map(|e| e.separator.clone()).unwrap_or_else(|| {
            if original.is_empty() {
                String::new()
            } else if original.ends_with("\r\n") {
                "\r\n".into()
            } else if original.ends_with('\n') {
                "\n".into()
            } else {
                "\n\n".into()
            }
        });
        let replacement = if kind == "markdown" {
            block.clone()
        } else {
            server_entry(&executable, &kind, &effective)
        };
        let after = edit(tool, &kind, &original, Some(&replacement), &separator)?;
        let content = current(tool, &kind, &after)?.ok_or("new entry not readable")?;
        entries.push(OwnedEntry {
            path: path.clone(),
            kind,
            content,
            separator,
            created: prior.map_or(before.is_none(), |e| e.created),
        });
        if after.len() as u64 > files::MAX_BYTES {
            return Err("generated configuration exceeds size limit".into());
        }
        changes.push(Mutation {
            path,
            before,
            after: Some(after.into_bytes()),
            executable: false,
        });
    }
    let receipt = Receipt {
        schema_version: 1,
        tool: tool.name.into(),
        version: tool.version.into(),
        agents,
        executable: executable.clone(),
        binary_digest,
        entries,
        gitflow_root: effective.gitflow_root,
        python: effective.python,
    };
    let path = receipt_path(tool, options);
    changes.push(Mutation {
        before: files::read(&path, files::MAX_BYTES)?,
        path,
        after: Some(serde_json::to_vec_pretty(&receipt).map_err(|e| e.to_string())?),
        executable: false,
    });
    files::apply(&changes, &options.state_dir(), options.dry_run)?;
    Ok(
        json!({"tool":tool.name,"status":if options.dry_run {"preview"} else {"installed"},"changed_files":changes.iter().filter(|c|c.before!=c.after).map(|c|&c.path).collect::<Vec<_>>(),"executable":executable,"path_action":"Add the executable's parent directory to your shell PATH if needed; PATH was not modified.","host_status":"configured_not_host_verified","note":"Reload the selected host. MCP exposes read-only context/instructions, not professional check approval."}),
    )
}

pub(crate) fn uninstall(tool: &Tool, options: &Options) -> Result<Value, String> {
    let Some(receipt) = load(tool, options)? else {
        return Ok(json!({"status":"not_installed","tool":tool.name}));
    };
    let mut changes = vec![];
    for entry in &receipt.entries {
        let before = files::read(&entry.path, files::MAX_BYTES)?;
        let original = text(before.as_deref())?;
        owned_content(
            tool,
            Some(entry),
            current(tool, &entry.kind, &original)?.as_deref(),
        )?;
        let updated = edit(tool, &entry.kind, &original, None, &entry.separator)?;
        let empty = updated.trim().is_empty()
            || entry.kind == "json"
                && json_config(&updated)?
                    .as_object()
                    .is_some_and(|m| m.is_empty());
        let after = if empty && entry.created {
            None
        } else {
            Some(updated.into_bytes())
        };
        changes.push(Mutation {
            path: entry.path.clone(),
            before,
            after,
            executable: false,
        });
    }
    if options.remove_binary {
        if let Some(expected) = &receipt.binary_digest {
            let requested = options.bin_dir.as_ref().map(|directory| {
                directory.join(format!("{}{}", tool.name, std::env::consts::EXE_SUFFIX))
            });
            if requested.as_ref().and_then(|path| path.canonicalize().ok())
                != receipt.executable.canonicalize().ok()
                || requested.is_none()
            {
                return Err("--remove-binary requires --bin-dir matching the owned executable; a project receipt cannot authorize arbitrary deletion".into());
            }
            let before = files::read(&receipt.executable, 512 * 1024 * 1024)?
                .ok_or("owned executable missing")?;
            if files::digest(&before) != *expected {
                return Err("owned executable changed; preserving it".into());
            }
            if receipt.executable.canonicalize().ok()
                == std::env::current_exe()
                    .ok()
                    .and_then(|p| p.canonicalize().ok())
            {
                return Err(
                    "run uninstall --remove-binary from a different source executable".into(),
                );
            }
            changes.push(Mutation {
                path: receipt.executable.clone(),
                before: Some(before),
                after: None,
                executable: true,
            });
        }
    }
    let path = receipt_path(tool, options);
    // 保留二进制时保留最小归属记录，后续可安全移除或重新接入。
    let after = if receipt.binary_digest.is_some() && !options.remove_binary {
        let kept = Receipt {
            agents: vec![],
            entries: vec![],
            ..receipt.clone()
        };
        Some(serde_json::to_vec_pretty(&kept).map_err(|e| e.to_string())?)
    } else {
        None
    };
    changes.push(Mutation {
        before: files::read(&path, files::MAX_BYTES)?,
        path,
        after,
        executable: false,
    });
    files::apply(&changes, &options.state_dir(), options.dry_run)?;
    Ok(
        json!({"tool":tool.name,"status":if options.dry_run {"preview"} else {"uninstalled"},"binary_retained":receipt.binary_digest.is_some()&&!options.remove_binary,"changed_files":changes.iter().filter(|c|c.before!=c.after).map(|c|&c.path).collect::<Vec<_>>()}),
    )
}

pub(crate) fn doctor(tool: &Tool, options: &Options) -> Result<Value, String> {
    let mut problems = vec![];
    if options.state_dir().join(".lock").exists() {
        problems.push("installation lock exists; inspect unfinished operation".to_owned());
    }
    let receipt = load(tool, options)?;
    if let Some(receipt) = &receipt {
        for entry in &receipt.entries {
            let content = text(files::read(&entry.path, files::MAX_BYTES)?.as_deref())?;
            if current(tool, &entry.kind, &content)?.as_deref() != Some(&entry.content) {
                problems.push(format!("owned entry changed: {}", entry.path.display()));
            }
            if entry.kind == "markdown" && instructions::render(tool, options)? != entry.content {
                problems.push("instructions stale; run instructions sync".into());
            }
        }
        if !receipt.executable.is_file() {
            problems.push("configured executable missing".into());
        }
        if let Some(expected) = &receipt.binary_digest {
            if files::read(&receipt.executable, 512 * 1024 * 1024)?
                .as_ref()
                .map(|b| files::digest(b))
                .as_ref()
                != Some(expected)
            {
                problems.push("installed executable digest mismatch".into());
            }
        }
        if receipt.version != tool.version {
            problems.push("installed version differs from running version".into());
        }
    } else {
        problems.push("not installed in selected scope".into());
    }
    Ok(
        json!({"tool":tool.name,"status":if problems.is_empty(){"configured"}else{"incomplete"},"problems":problems,"host_status":"not_verified","quality_decision":"not_evaluated"}),
    )
}
