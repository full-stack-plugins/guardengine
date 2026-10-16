use crate::{CompileContext, ContractLayer};
use guard_protocol::Action;
/// 只读编译请求；不接受动态验证器配置或来源信任声明。无 Java 对应类型。
#[derive(Clone, Debug)]
pub struct CompileRequest {
    /// 三个层次必须各有且仅有一个。
    pub layers: Vec<ContractLayer>,
    /// 任务归属及已有授权引用。
    pub context: CompileContext,
    /// 拟准备的精确工程动作，不在此执行。
    pub action: Action,
}
