use serde::{Deserialize, Serialize};

/// 确定性关系图断言，不执行表达式或脚本；原生 Rust 协议，无 Java 对应类。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum GuardAssertion {
    /// 禁止精确直接边。
    ForbidRelation {
        subject: String,
        predicate: String,
        object: String,
    },
    /// 禁止经过一条或多条边到达目标。
    ForbidPath {
        subject: String,
        predicate: String,
        object: String,
    },
    /// 源实体只允许直接依赖给定对象列表，空列表表示不得有出边。
    AllowedDependencies {
        subject: String,
        predicate: String,
        objects: Vec<String>,
    },
    /// 必须有精确直接边。
    RequireRelation {
        subject: String,
        predicate: String,
        object: String,
    },
    /// 指定谓词的完整图不得有环。
    Acyclic { predicate: String },
}

impl GuardAssertion {
    /// 返回当前断言的关系谓词；输入为断言自身，输出借用字符串。
    pub fn predicate(&self) -> &str {
        match self {
            Self::ForbidRelation { predicate, .. }
            | Self::ForbidPath { predicate, .. }
            | Self::AllowedDependencies { predicate, .. }
            | Self::RequireRelation { predicate, .. }
            | Self::Acyclic { predicate } => predicate,
        }
    }
    /// 返回需要存在于覆盖声明中的实体，便于检查拼写和适用范围。
    pub fn entities(&self) -> Vec<&str> {
        match self {
            Self::ForbidRelation {
                subject, object, ..
            }
            | Self::ForbidPath {
                subject, object, ..
            }
            | Self::RequireRelation {
                subject, object, ..
            } => vec![subject, object],
            Self::AllowedDependencies {
                subject, objects, ..
            } => std::iter::once(subject.as_str())
                .chain(objects.iter().map(String::as_str))
                .collect(),
            Self::Acyclic { .. } => vec![],
        }
    }
}
