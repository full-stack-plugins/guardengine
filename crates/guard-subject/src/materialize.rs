use crate::{
    CaptureBinding, CapturedSubject, MaterializedSubject, Selection, capture,
    git_client::GitClient, manifest, materialization_io, paths,
};
use std::{fs, path::Path};
/// 从相同真实候选创建私有输入副本；不写原仓index/ref/objects，不提供OS沙箱。
/// project为真实仓库根，binding和selection必须与previous冻结捕获完全一致。
/// scratch_parent必须显式存在且在原仓及Git元数据外；返回对象拥有临时目录并在Drop时清理。
pub fn materialize(
    project: &Path,
    binding: &CaptureBinding,
    selection: &Selection,
    previous: &CapturedSubject,
    scratch_parent: &Path,
) -> Result<MaterializedSubject, String> {
    if &previous.binding != binding || &previous.selection != selection {
        return Err("materialization binding or selection differs".into());
    }
    let project = paths::safe_existing(project)?;
    let parent = paths::safe_existing(scratch_parent)?;
    if !parent.is_dir() || parent.starts_with(&project) {
        return Err("scratch parent must be outside original project".into());
    }
    let inspect = GitClient::inspect(project.clone())?;
    for metadata in inspect.metadata_directories()? {
        if parent.starts_with(metadata) {
            return Err("scratch parent must be outside Git metadata".into());
        }
    }
    let scratch = tempfile::Builder::new()
        .prefix("guard-materialized-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(scratch.path(), fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    let client = GitClient::new(project.clone(), scratch.path())?;
    if capture::once(&client, binding, selection)? != *previous
        || capture::once(&client, binding, selection)? != *previous
    {
        return Err("candidate changed before materialization".into());
    }
    let root = scratch.path().join("inputs");
    fs::create_dir(&root).map_err(|e| e.to_string())?;
    let root = paths::safe_existing(&root)?;
    for entry in &previous.manifest {
        materialization_io::path(&root, &entry.path.to_bytes())?;
        let bytes = match &entry.object_id {
            Some(oid) => crate::git_object::read(&client, "blob", oid, manifest::BYTE_LIMIT)?,
            None => materialization_io::read(&project, entry)?,
        };
        if manifest::digest(&bytes)? != entry.content_digest {
            return Err("candidate blob differs from frozen input".into());
        }
        materialization_io::write(&root, entry, &bytes)?;
    }
    if capture::once(&client, binding, selection)? != *previous {
        return Err("candidate changed during materialization".into());
    }
    let material = MaterializedSubject {
        _scratch: scratch,
        root,
        captured: previous.clone(),
    };
    material.verify_inputs()?;
    Ok(material)
}
