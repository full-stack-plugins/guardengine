use std::path::PathBuf;

/// 已预检的单文件变更，保留原字节以支持条件回滚。
pub(crate) struct Mutation {
    pub path: PathBuf,
    pub before: Option<Vec<u8>>,
    pub after: Option<Vec<u8>>,
    pub executable: bool,
}
