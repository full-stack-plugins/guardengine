/// 准备状态，与运行通过和授权无关。无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanStatus {
    /// 义务目标均绑定真实输入，可以交给受管执行层进一步验证。
    Ready,
    /// 保留全部义务，但缺目标或受保护输入不符，不能生成运行计划。
    Incomplete,
}
