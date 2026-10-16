use crate::{Digest, Identifier, ProducerIdentity};
use serde::{Deserialize, Serialize};
/// 固定构建运行与定义/调用/输入/工具身份；字段本身不证明可信 CI。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReleaseBuildIdentity {
    /// 精确运行及尝试标识，语义由独立生产者核验，不从名称推导信任。
    pub build_id: Identifier,
    /// 构建定义原件摘要，避免仅绑定可变 workflow 路径。
    pub definition_digest: Digest,
    /// 原始构建调用参数/配置的规范清单摘要，不执行该内容。
    pub invocation_digest: Digest,
    /// 全部构建输入清单摘要，包含源码以外所需固定材料。
    pub input_manifest_digest: Digest,
    /// 构建器版本、制品、配置、适配器及执行环境的固定声明。
    pub producer: ProducerIdentity,
}
