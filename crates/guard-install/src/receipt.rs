use crate::owned_entry::OwnedEntry;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 安装归属记录，保留安装版本和外部适配器配置。
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Receipt {
    pub schema_version: u32,
    pub tool: String,
    pub version: String,
    pub agents: Vec<String>,
    pub executable: PathBuf,
    pub binary_digest: Option<String>,
    pub entries: Vec<OwnedEntry>,
    pub gitflow_root: Option<PathBuf>,
    pub python: Option<PathBuf>,
}
