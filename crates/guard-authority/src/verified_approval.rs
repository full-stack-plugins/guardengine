use crate::ApprovalMaterial;
use guard_protocol::{ApprovalReceipt, Digest, Identifier};
/// 仅能由真实核验器构造的本次核验结果；不提供Deserialize且不永久缓存有效性。无Java对应类型。
/// ```compile_fail
/// let _: guard_authority::VerifiedApproval = serde_json::from_str("{}").unwrap();
/// ```
#[derive(Clone, Debug)]
pub struct VerifiedApproval {
    pub(crate) receipt: ApprovalReceipt,
    pub(crate) material: ApprovalMaterial,
}
impl VerifiedApproval {
    /// 返回已核对的批准引用；裸引用不具授权资格。
    pub fn reference(&self) -> &Identifier {
        &self.receipt.receipt_id
    }
    /// 返回本次已核对的签发者。
    pub fn issuer(&self) -> &Identifier {
        &self.receipt.issuer
    }
    /// 返回精确合同摘要；不代表候选技术验收。
    pub fn contract_digest(&self) -> &Digest {
        &self.receipt.contract_digest
    }
    /// 返回已核对的批准材料；读取结果不延长有效期。
    pub fn material(&self) -> &ApprovalMaterial {
        &self.material
    }
}
