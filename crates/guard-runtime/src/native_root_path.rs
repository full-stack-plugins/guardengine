use std::path::{Path, PathBuf};
/// 将控制端生成的绝对候选根转成原生Git/Java兼容的等价路径；不接受GitPath或设备命名空间。
pub(crate) fn native_root_path(root: &Path) -> Result<PathBuf, String> {
    if !root.is_absolute() {
        return Err("absolute materialized root required".into());
    }
    let text = root.to_str().ok_or("non-Unicode materialized root")?;
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        for component in root.components() {
            if let Component::Normal(name) = component {
                let name = name.to_str().ok_or("non-Unicode root component")?;
                if name.ends_with(['.', ' '])
                    || name.contains(':')
                    || name.chars().any(char::is_control)
                {
                    return Err("materialized root cannot be represented without Windows path normalization".into());
                }
            }
        }
        match root.components().next() {
            Some(Component::Prefix(prefix)) => match prefix.kind() {
                Prefix::VerbatimDisk(_) => {
                    return Ok(PathBuf::from(
                        text.strip_prefix(r"\\?\")
                            .ok_or("unsupported verbatim drive root")?,
                    ));
                }
                Prefix::VerbatimUNC(_, _) => {
                    return Ok(PathBuf::from(format!(
                        r"\\{}",
                        text.strip_prefix(r"\\?\UNC\")
                            .ok_or("unsupported verbatim UNC root")?
                    )));
                }
                Prefix::Disk(_) | Prefix::UNC(_, _) => {}
                _ => return Err("unsupported device namespace for materialized root".into()),
            },
            _ => return Err("unsupported Windows materialized root".into()),
        }
    }
    Ok(PathBuf::from(text))
}

#[cfg(test)]
mod tests {
    use super::native_root_path;
    use std::path::{Path, PathBuf};
    #[test]
    fn rejects_relative_roots_without_touching_filesystem() {
        assert!(native_root_path(Path::new("relative-root")).is_err());
    }
    #[cfg(windows)]
    #[test]
    fn windows_verbatim_drive_and_unc_are_equivalent_native_paths() {
        assert_eq!(
            native_root_path(Path::new(r"\\?\E:\私有根\inputs")).unwrap(),
            PathBuf::from(r"E:\私有根\inputs")
        );
        assert_eq!(
            native_root_path(Path::new(r"\\?\UNC\server\share\私有根")).unwrap(),
            PathBuf::from(r"\\server\share\私有根")
        );
        assert!(native_root_path(Path::new(r"\\.\PIPE\unsafe")).is_err());
        assert!(native_root_path(Path::new(r"\\?\E:\ambiguous.\inputs")).is_err());
        assert!(native_root_path(Path::new(r"\\?\E:\ambiguous \inputs")).is_err());
    }
    #[cfg(not(windows))]
    #[test]
    fn unix_absolute_root_bytes_are_unchanged() {
        assert_eq!(
            native_root_path(Path::new(r"/tmp/私有根\literal")).unwrap(),
            PathBuf::from(r"/tmp/私有根\literal")
        );
    }
}
