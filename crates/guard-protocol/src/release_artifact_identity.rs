use crate::{ArtifactMediaType, ArtifactName, Digest};
use serde::{Deserialize, Serialize};
/// 精确制品原始字节身份；声明不证明真实文件已经存在或被采集。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReleaseArtifactIdentity {
    /// 非路径逻辑名称，不能作为任意输出路径。
    pub name: ArtifactName,
    /// 原始制品字节 SHA256，不等同 Git tree/commit OID。
    pub content_digest: Digest,
    /// 精确字节数，限制为安全整数，零字节是否适用由专业规则判断。
    pub size: u64,
    /// 不含参数的受限小写 MIME 标识。
    pub media_type: ArtifactMediaType,
}
