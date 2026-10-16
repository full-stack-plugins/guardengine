use serde::{Deserialize, Serialize};

/// 分析器明确覆盖的实体、关系与语义范围；本项目原生协议，无 Java 对应类。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AnalysisCoverage {
    /// 实际发现的完整实体集合，包含没有依赖边的模块。
    pub entities: Vec<String>,
    /// 分析器支持的关系谓词。
    pub predicates: Vec<String>,
    /// 人类可读的覆盖边界，不能把声明图说成运行时图。
    pub scope: String,
}
