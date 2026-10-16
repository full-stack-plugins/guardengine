use std::{collections::HashSet, path::PathBuf};

/// 安装和只读上下文参数；路径始终为调用者显式选择的目标。
#[derive(Clone, Debug)]
pub struct Options {
    /// 目标项目绝对路径。
    pub project: PathBuf,
    /// 用户配置根目录，可用 --home 隔离测试。
    pub home: PathBuf,
    /// 是否使用用户级安装。
    pub user: bool,
    /// 指定的宿主列表；None 表示复用安装记录。
    pub agents: Option<Vec<String>>,
    /// 可执行程序复制位置。
    pub bin_dir: Option<PathBuf>,
    /// 是否只预览。
    pub dry_run: bool,
    /// 卸载时是否删除归属已验证的二进制副本。
    pub remove_binary: bool,
    /// 显式信任的 GitFlow 插件根。
    pub gitflow_root: Option<PathBuf>,
    /// 用于 GitFlow 的 Python 可执行文件。
    pub python: Option<PathBuf>,
    /// 显式选择原生规格体系，不初始化或迁移。
    pub spec_system: Option<String>,
    /// 已有原生规格目标的项目相对路径。
    pub spec_target: Option<String>,
    pub(crate) custom_home: bool,
}

impl Options {
    pub(crate) fn parse(args: &[String]) -> Result<Self, String> {
        let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
        let mut value = Self {
            project: cwd.clone(),
            home: std::env::var_os("USERPROFILE")
                .or_else(|| std::env::var_os("HOME"))
                .map(PathBuf::from)
                .unwrap_or_else(|| cwd.clone()),
            user: false,
            agents: None,
            bin_dir: None,
            dry_run: false,
            remove_binary: false,
            gitflow_root: None,
            python: None,
            spec_system: None,
            spec_target: None,
            custom_home: false,
        };
        let mut i = 0;
        let mut seen = HashSet::new();
        while i < args.len() {
            let (key, inline) = args[i]
                .split_once('=')
                .map_or((args[i].as_str(), None), |(a, b)| (a, Some(b)));
            if !seen.insert(key) {
                return Err(format!("duplicate option: {key}"));
            }
            if matches!(key, "--dry-run" | "--remove-binary" | "--integration") {
                if inline.is_some() {
                    return Err(format!("flag takes no value: {key}"));
                }
                match key {
                    "--dry-run" => value.dry_run = true,
                    "--remove-binary" => value.remove_binary = true,
                    _ => {}
                }
            } else {
                let argument = match inline {
                    Some(v) => v,
                    None => {
                        i += 1;
                        args.get(i)
                            .ok_or_else(|| format!("missing value for {key}"))?
                    }
                };
                if argument.is_empty() || argument.starts_with("--") {
                    return Err(format!("invalid value for {key}"));
                }
                let absolute = |s: &str| {
                    let p = PathBuf::from(s);
                    if p.is_absolute() { p } else { cwd.join(p) }
                };
                match key {
                    "--project" => value.project = absolute(argument),
                    "--home" => {
                        value.home = absolute(argument);
                        value.custom_home = true;
                    }
                    "--scope" => {
                        value.user = match argument {
                            "user" => true,
                            "project" => false,
                            _ => return Err("scope must be project or user".into()),
                        }
                    }
                    "--agents" => {
                        let list = if argument == "none" {
                            vec![]
                        } else {
                            argument.split(',').map(str::to_owned).collect::<Vec<_>>()
                        };
                        let mut unique = HashSet::new();
                        if list.iter().any(|a| {
                            !matches!(a.as_str(), "codex" | "claude" | "gemini")
                                || !unique.insert(a)
                        }) {
                            return Err("agents must be unique codex,claude,gemini or none".into());
                        }
                        value.agents = Some(list);
                    }
                    "--bin-dir" => value.bin_dir = Some(absolute(argument)),
                    "--gitflow-root" => value.gitflow_root = Some(absolute(argument)),
                    "--python" => value.python = Some(absolute(argument)),
                    "--spec-system" => value.spec_system = Some(argument.to_owned()),
                    "--spec-target" => value.spec_target = Some(argument.to_owned()),
                    "--format" if argument == "json" => {}
                    _ => return Err(format!("unknown option: {key}")),
                }
            }
            i += 1;
        }
        if !value.project.is_dir() {
            return Err("project must be an existing directory".into());
        }
        value.project = value.project.canonicalize().map_err(|e| e.to_string())?;
        Ok(value)
    }

    pub(crate) fn state_dir(&self) -> PathBuf {
        if self.user {
            self.home.join(".guard-install")
        } else {
            self.project.join(".guard-install")
        }
    }

    pub(crate) fn targets(&self, agents: &[String]) -> Vec<(PathBuf, String)> {
        let mut result = vec![];
        for agent in agents {
            let root = if self.user { &self.home } else { &self.project };
            match agent.as_str() {
                "codex" => {
                    let config = if self.user && !self.custom_home {
                        std::env::var_os("CODEX_HOME")
                            .map(PathBuf::from)
                            .unwrap_or_else(|| root.join(".codex"))
                    } else {
                        root.join(".codex")
                    };
                    result.push((
                        if self.user {
                            config.join("AGENTS.md")
                        } else {
                            root.join("AGENTS.md")
                        },
                        "markdown".into(),
                    ));
                    result.push((config.join("config.toml"), "toml".into()));
                }
                "claude" => {
                    result.push((
                        if self.user {
                            root.join(".claude/CLAUDE.md")
                        } else {
                            root.join("CLAUDE.md")
                        },
                        "markdown".into(),
                    ));
                    result.push((
                        root.join(if self.user {
                            ".claude.json"
                        } else {
                            ".mcp.json"
                        }),
                        "json".into(),
                    ));
                }
                "gemini" => {
                    result.push((
                        root.join(if self.user {
                            ".gemini/GEMINI.md"
                        } else {
                            "GEMINI.md"
                        }),
                        "markdown".into(),
                    ));
                    result.push((root.join(".gemini/settings.json"), "json".into()));
                }
                _ => {}
            }
        }
        result
    }
}
