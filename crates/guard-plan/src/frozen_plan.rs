use guard_protocol::{Digest, VerificationPlan};
/// 只能由本库验证真实候选后产生的不可变运行计划；不接受 Deserialize。无 Java 对应类型。
#[derive(Clone, Debug)]
pub struct FrozenPlan {
    pub(crate) verification: VerificationPlan,
    pub(crate) digest: Digest,
}
impl FrozenPlan {
    /// 返回只读协议投影；复制或解析这份 JSON 不能重建本类型或执行权限。
    pub fn verification(&self) -> &VerificationPlan {
        &self.verification
    }
    /// 返回包括义务、精确目标、绑定和预算的运行计划摘要。
    pub fn digest(&self) -> &Digest {
        &self.digest
    }
    /// 仅比较两个本库构造计划的全部身份，不证明外部报告签名或运行状态。
    pub fn equivalent_inputs(&self, previous: &FrozenPlan) -> bool {
        self.digest == previous.digest
    }
}
