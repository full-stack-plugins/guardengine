use crate::{Digest, ExactTarget, GitPath, Identifier};
use serde::{Deserialize, Serialize};

/// 批准契约选择的仓库/路径/符号范围；普通源码在计划阶段绑定当前内容。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TargetSelection {
    /// 已批准的仓库身份声明。
    pub repository_id: Identifier,
    /// 无损原始 Git 相对路径。
    pub path: GitPath,
    /// 可选精确符号选择器，绑定时不能扩大或替换。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<Identifier>,
    /// 受保护输入可固定原始内容摘要；缺省时不把普通候选内容写入批准契约。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protected_digest: Option<Digest>,
}
impl TargetSelection {
    /// 在不改范围的前提下绑定实际候选摘要；受保护内容变化返回错误。
    /// 参数由独立输入采集端提供，本方法不证明文件存在、批准或摘要来源可信。
    pub fn bind(&self, input_digest: Digest) -> Result<ExactTarget, String> {
        if self
            .protected_digest
            .as_ref()
            .is_some_and(|expected| expected != &input_digest)
        {
            return Err("protected target digest cannot be rebound".into());
        }
        Ok(ExactTarget {
            repository_id: self.repository_id.clone(),
            path: self.path.clone(),
            selector: self.selector.clone(),
            input_digest,
        })
    }
}
