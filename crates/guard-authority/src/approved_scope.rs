use guard_protocol::{Action, Digest, Identifier, TargetSelection};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
/// 本profile精确批准范围；无Java对应类型，不支持glob或前缀包含。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApprovedScope {
    pub repository_id: Identifier,
    pub task_id: Identifier,
    pub actions: Vec<Action>,
    pub targets: Vec<TargetSelection>,
}
impl ApprovedScope {
    /// 校验非空且唯一的精确范围，返回JCS身份；保留原始GitPath字节。
    pub fn digest(&self) -> Result<Digest, String> {
        if self.actions.is_empty()
            || self.actions.len() > 32
            || self.targets.is_empty()
            || self.targets.len() > 4096
        {
            return Err("scope budget or empty scope".into());
        }
        let mut actions = BTreeSet::new();
        for action in &self.actions {
            if !actions.insert(serde_json::to_vec(action).map_err(|e| e.to_string())?) {
                return Err("duplicate scope action".into());
            }
        }
        let mut paths = BTreeSet::new();
        for target in &self.targets {
            if target.repository_id != self.repository_id {
                return Err("target outside repository".into());
            }
            let key = guard_protocol::canonicalize(
                &serde_json::to_vec(target).map_err(|e| e.to_string())?,
            )?;
            if !paths.insert(key) {
                return Err("duplicate scope target".into());
            }
            // 选择器仅精确标识；未知模式不得被误解释成目录或glob。
            if target
                .selector
                .as_ref()
                .is_some_and(|v| v.as_str().contains(['*', '?', '[', ']']))
            {
                return Err("selector patterns unsupported".into());
            }
        }
        let actions = actions
            .into_iter()
            .map(|bytes| {
                serde_json::from_slice::<serde_json::Value>(&bytes).map_err(|e| e.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let targets = paths
            .into_iter()
            .map(|bytes| {
                serde_json::from_slice::<serde_json::Value>(&bytes).map_err(|e| e.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        guard_protocol::digest_json(&serde_json::to_vec(&serde_json::json!({"repositoryId":self.repository_id,"taskId":self.task_id,"actions":actions,"targets":targets})).map_err(|e|e.to_string())?)
    }
}
