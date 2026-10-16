use crate::{Digest, Extensions, Identifier, SchemaVersion};
use serde::{Deserialize, Serialize};
/// 必须由受信注册表核实的生产者及运行环境声明。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProducerIdentity {
    /// 冻结协议字段 `schemaVersion`；语义见本 crate 协议表。
    pub schema_version: SchemaVersion,
    /// 冻结协议字段 `producerId`；语义见本 crate 协议表。
    pub producer_id: Identifier,
    /// 冻结协议字段 `version`；语义见本 crate 协议表。
    pub version: Identifier,
    /// 冻结协议字段 `artifactDigest`；语义见本 crate 协议表。
    pub artifact_digest: Digest,
    /// 冻结协议字段 `configDigest`；语义见本 crate 协议表。
    pub config_digest: Digest,
    /// 冻结协议字段 `adapterVersion`；语义见本 crate 协议表。
    pub adapter_version: Identifier,
    /// 冻结协议字段 `executionEnvironmentDigest`；语义见本 crate 协议表。
    pub execution_environment_digest: Digest,
    /// 冻结协议字段 `extensions`；语义见本 crate 协议表。
    pub extensions: Extensions,
}
