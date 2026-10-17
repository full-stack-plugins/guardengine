use crate::{PolicyInput, PolicyPackage, paths, policy_package::hash};
use guard_protocol::Digest;
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};
/// 仅含冻结包、能力与输入的私有副本；不是OS沙箱。无Java对应类型。
pub(crate) struct PolicyWorkspace {
    owner: tempfile::TempDir,
    files: BTreeMap<String, Digest>,
}
impl PolicyWorkspace {
    pub(crate) fn create(
        parent: &Path,
        package: &PolicyPackage,
        input: &PolicyInput,
    ) -> Result<Self, String> {
        let parent = paths::safe(parent)?;
        let owner = tempfile::Builder::new()
            .prefix("guard-policy-")
            .tempdir_in(parent)
            .map_err(|e| e.to_string())?;
        let mut files = BTreeMap::new();
        let mut all = package
            .files()
            .iter()
            .map(|(p, b)| (format!("bundle/{p}"), b.clone()))
            .collect::<BTreeMap<_, _>>();
        all.insert("capabilities.json".into(), package.capabilities().to_vec());
        all.insert("input.json".into(), input.bytes()?);
        for (name, bytes) in all {
            let path = owner.path().join(&name);
            fs::create_dir_all(path.parent().ok_or("private path parent")?)
                .map_err(|e| e.to_string())?;
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .map_err(|e| e.to_string())?;
            file.write_all(&bytes)
                .and_then(|_| file.sync_all())
                .map_err(|e| e.to_string())?;
            files.insert(name, hash(&bytes));
        }
        let workspace = Self { owner, files };
        workspace.verify()?;
        Ok(workspace)
    }
    pub(crate) fn root(&self) -> &Path {
        self.owner.path()
    }
    pub(crate) fn path(&self, name: &str) -> PathBuf {
        self.root().join(name)
    }
    pub(crate) fn verify(&self) -> Result<(), String> {
        let mut stack = vec![self.root().to_path_buf()];
        let mut found = BTreeMap::new();
        while let Some(dir) = stack.pop() {
            paths::safe(&dir)?;
            for item in fs::read_dir(&dir).map_err(|e| e.to_string())? {
                let path = item.map_err(|e| e.to_string())?.path();
                paths::safe(&path)?;
                if path.is_dir() {
                    stack.push(path)
                } else {
                    let name = path
                        .strip_prefix(self.root())
                        .map_err(|e| e.to_string())?
                        .to_str()
                        .ok_or("Unicode private policy path required")?
                        .replace('\\', "/");
                    let bytes = paths::read(&path, 1024 * 1024)?;
                    if found.insert(name, hash(&bytes)).is_some() {
                        return Err("duplicate private input path".into());
                    }
                }
                if found.len() + stack.len() > 128 {
                    return Err("private policy inventory exceeds budget".into());
                }
            }
        }
        if found != self.files {
            return Err("private policy or input changed during execution".into());
        }
        Ok(())
    }
}
