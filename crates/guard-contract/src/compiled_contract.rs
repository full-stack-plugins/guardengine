use crate::{CompileContext, LayerIdentity, VerifierPin};
use guard_protocol::{Action, ContractObligation, Digest, Identifier, RuleDefinition};
/// 私有不可变编译产物，不提供Deserialize或公开字段修改。无 Java 对应类型。
#[derive(Clone, Debug)]
pub struct CompiledContract {
    pub(crate) digest: Digest,
    pub(crate) context: CompileContext,
    pub(crate) action: Action,
    pub(crate) layers: Vec<LayerIdentity>,
    pub(crate) rules: Vec<RuleDefinition>,
    pub(crate) obligations: Vec<ContractObligation>,
    pub(crate) verifiers: Vec<VerifierPin>,
    pub(crate) authorization_ref: Identifier,
    pub(crate) schema_digest: Digest,
    pub(crate) sources: Vec<guard_protocol::SourceRef>,
    pub(crate) approvals: Vec<crate::ApprovalAttestation>,
    pub(crate) authorization_authority: Identifier,
}
impl CompiledContract {
    /// 返回精确编译身份，后续任务显式绑定它。
    pub fn digest(&self) -> &Digest {
        &self.digest
    }
    /// 返回被批准的任务归属，不代表执行许可。
    pub fn context(&self) -> &CompileContext {
        &self.context
    }
    /// 返回拟准备的动作。
    pub fn action(&self) -> Action {
        self.action
    }
    /// 返回已批准层次身份。
    pub fn layers(&self) -> &[LayerIdentity] {
        &self.layers
    }
    /// 返回所有有效规则，不按扫描结果删减。
    pub fn rules(&self) -> &[RuleDefinition] {
        &self.rules
    }
    /// 返回全部义务，后续计划不能从工具结果倒推。
    pub fn obligations(&self) -> &[ContractObligation] {
        &self.obligations
    }
    /// 返回验证器与OPA精确pin。
    pub fn verifiers(&self) -> &[VerifierPin] {
        &self.verifiers
    }
    /// 返回已核验的既有任务授权引用，不是新ExecutionGrant。
    pub fn authorization_ref(&self) -> &Identifier {
        &self.authorization_ref
    }
    /// 返回实际使用的协议Schema原件摘要。
    pub fn schema_digest(&self) -> &Digest {
        &self.schema_digest
    }
}

impl CompiledContract {
    /// 返回实际校验的不可变来源引用，原始字节未写入公共产物。
    pub fn source_refs(&self) -> &[guard_protocol::SourceRef] {
        &self.sources
    }
    /// 返回显式受信端口的逐层核验记录，不代表本 crate 完成密码学验签。
    pub fn approval_attestations(&self) -> &[crate::ApprovalAttestation] {
        &self.approvals
    }
    /// 返回既有任务授权的外部核验者身份。
    pub fn authorization_authority(&self) -> &Identifier {
        &self.authorization_authority
    }
}
