use serde::Serialize;
/// 结构 Schema 的精确错误位置，不回显私有原件内容。无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SchemaIssue {
    /// 实例中的 JSON Pointer。
    pub instance_path: String,
    /// Schema 中失败关键词的位置。
    pub schema_path: String,
}
