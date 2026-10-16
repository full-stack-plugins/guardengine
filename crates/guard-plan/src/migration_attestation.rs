use guard_protocol::{Digest, Identifier};
/// 独立迁移端口的结果；由受保护调用方配置端口，不从候选 JSON 选择权威。无 Java 对应类型。
#[derive(Clone, Debug)]
pub struct MigrationAttestation {
    /// 外部核验者身份，本库不自行验证密码学权威。
    pub authority_id: Identifier,
    /// 可审计的迁移批准引用。
    pub approval_ref: Identifier,
    /// 必须精确匹配旧/新冻结绑定的迁移请求摘要。
    pub request_digest: Digest,
}
