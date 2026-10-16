use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
/// 只执行固定 plumbing argv；私有 index/objects 允许 Git 计算树而不写调用方状态。无 Java 对应类型。
pub(crate) struct GitClient {
    pub(crate) root: PathBuf,
    pub(crate) objects: PathBuf,
    pub(crate) original_objects: PathBuf,
    pub(crate) private_index: PathBuf,
    deadline: Instant,
}
impl GitClient {
    pub(crate) fn new(root: PathBuf, scratch: &Path) -> Result<Self, String> {
        let mut client = Self::inspect(root)?;
        client.objects = scratch.join("objects");
        fs::create_dir(&client.objects).map_err(|e| e.to_string())?;
        client.private_index = scratch.join("index");
        Ok(client)
    }
    /// 仅检查既有仓库和元数据位置，临时目录尚未创建时可安全检查其parent。
    pub(crate) fn inspect(root: PathBuf) -> Result<Self, String> {
        let mut client = Self {
            root,
            objects: PathBuf::new(),
            original_objects: PathBuf::new(),
            private_index: PathBuf::new(),
            deadline: Instant::now() + Duration::from_secs(60),
        };
        let bare = client.text(&["rev-parse", "--is-bare-repository"])?;
        if bare != "false" {
            return Err("bare repositories unsupported".into());
        }
        let top = client.text(&["rev-parse", "--show-toplevel"])?;
        let top = crate::paths::safe_existing(Path::new(&top))?;
        if top != client.root {
            return Err("project must be the exact repository root".into());
        }
        let original = client.text(&["rev-parse", "--git-path", "objects"])?;
        client.original_objects = crate::paths::safe_existing(&client.root.join(original))?;
        let path = client
            .original_objects
            .to_str()
            .ok_or("non-UTF8 Git metadata directory unsupported")?;
        if path.contains(['\n', '\r', '\0', if cfg!(windows) { ';' } else { ':' }]) {
            return Err("Git metadata directory cannot be safely represented as alternate".into());
        }
        Ok(client)
    }
    pub(crate) fn metadata_directories(&self) -> Result<Vec<PathBuf>, String> {
        let mut paths = vec![self.original_objects.clone()];
        for args in [
            ["rev-parse", "--absolute-git-dir"],
            ["rev-parse", "--git-common-dir"],
        ] {
            let path = self.text(&args)?;
            paths.push(crate::paths::safe_existing(&self.root.join(path))?);
        }
        Ok(paths)
    }
    pub(crate) fn text(&self, args: &[&str]) -> Result<String, String> {
        let value = self.run(args, 1024 * 1024, false)?;
        Ok(std::str::from_utf8(&value)
            .map_err(|_| "Git metadata must be UTF-8")?
            .trim_end_matches(['\r', '\n'])
            .into())
    }
    pub(crate) fn run(
        &self,
        args: &[&str],
        limit: usize,
        private: bool,
    ) -> Result<Vec<u8>, String> {
        if Instant::now() >= self.deadline {
            return Err("Git capture deadline exceeded".into());
        }
        let mut command = Command::new("git");
        command
            .arg("--no-optional-locks")
            .arg("-c")
            .arg("core.fsmonitor=false")
            .arg("-c")
            .arg("gc.auto=0")
            .arg("-c")
            .arg("maintenance.auto=false")
            .arg("-C")
            .arg(&self.root)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (name, _) in std::env::vars_os() {
            if name
                .to_string_lossy()
                .to_ascii_uppercase()
                .starts_with("GIT_")
            {
                command.env_remove(name);
            }
        }
        command
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env(
                "GIT_CONFIG_GLOBAL",
                if cfg!(windows) { "NUL" } else { "/dev/null" },
            )
            .env("GIT_NO_REPLACE_OBJECTS", "1")
            .env("GIT_NO_LAZY_FETCH", "1")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_OPTIONAL_LOCKS", "0");
        if private {
            if !self.objects.is_absolute() || !self.private_index.is_absolute() {
                return Err("private Git paths have not been initialized".into());
            }
            command
                .env("GIT_OBJECT_DIRECTORY", environment_path(&self.objects)?)
                .env(
                    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
                    environment_path(&self.original_objects)?,
                )
                .env("GIT_INDEX_FILE", environment_path(&self.private_index)?);
        }
        let mut child = command
            .spawn()
            .map_err(|e| format!("Git unavailable: {e}"))?;
        let stdout = child.stdout.take().ok_or("Git stdout missing")?;
        let stderr = child.stderr.take().ok_or("Git stderr missing")?;
        let (sender, receiver) = mpsc::channel();
        for (kind, stream, budget) in [
            (0, Box::new(stdout) as Box<dyn Read + Send>, limit),
            (1, Box::new(stderr) as Box<dyn Read + Send>, 65536),
        ] {
            let sender = sender.clone();
            thread::spawn(move || {
                let mut buffer = Vec::new();
                let result = stream
                    .take(budget as u64 + 1)
                    .read_to_end(&mut buffer)
                    .map_err(|e| e.to_string())
                    .and_then(|_| {
                        if buffer.len() > budget {
                            Err("Git output limit exceeded".into())
                        } else {
                            Ok(buffer)
                        }
                    });
                let _ = sender.send((kind, result));
            });
        }
        drop(sender);
        let mut output = None;
        let mut errors = None;
        loop {
            if Instant::now() >= self.deadline {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Git capture deadline exceeded".into());
            }
            while let Ok((kind, value)) = receiver.try_recv() {
                match value {
                    Err(error) => {
                        let _ = child.kill();
                        let _ = child.wait();
                        return Err(error);
                    }
                    Ok(bytes) => {
                        if kind == 0 {
                            output = Some(bytes);
                        } else {
                            errors = Some(bytes);
                        }
                    }
                }
            }
            if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                if output.is_some() && errors.is_some() {
                    if !status.success() {
                        return Err(format!(
                            "Git {:?} failed: {}",
                            args,
                            String::from_utf8_lossy(errors.as_deref().unwrap_or_default())
                        ));
                    }
                    return output.ok_or_else(|| "Git stdout missing".into());
                }
            }
            thread::sleep(Duration::from_millis(2));
        }
    }
}

// Windows canonical路径使用verbatim前缀，但Git的index.lock环境路径不接受该形式。
// 仅转换等价的绝对私有元数据路径；原始Git文件名和候选身份不在此转换。
fn environment_path(path: &Path) -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        if let Some(Component::Prefix(prefix)) = path.components().next() {
            let text = path
                .to_str()
                .ok_or("non-Unicode private Git path unsupported")?;
            match prefix.kind() {
                Prefix::VerbatimDisk(_) => {
                    return Ok(PathBuf::from(
                        text.strip_prefix(r"\\?\")
                            .ok_or("invalid verbatim drive path")?,
                    ));
                }
                Prefix::VerbatimUNC(_, _) => {
                    return Ok(PathBuf::from(format!(
                        r"\\{}",
                        text.strip_prefix(r"\\?\UNC\")
                            .ok_or("invalid verbatim UNC path")?
                    )));
                }
                Prefix::Verbatim(_) | Prefix::DeviceNS(_) => {
                    return Err("private Git device path unsupported".into());
                }
                _ => {}
            }
        }
    }
    Ok(path.to_path_buf())
}
