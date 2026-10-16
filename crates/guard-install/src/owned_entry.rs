use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 单个归属条目；只拥有提示区块或 MCP 节点，不拥有整个用户配置。
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct OwnedEntry {
    pub path: PathBuf,
    pub kind: String,
    pub content: String,
    pub separator: String,
    pub created: bool,
}
