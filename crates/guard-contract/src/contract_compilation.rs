use crate::{CompilationDiagnostic, CompilationStatus, CompiledContract};
use guard_protocol::{ContractObligation, RuleDefinition};
/// 编译结果即便阻塞也保留已读取规则与义务，不回退为空规则成功。无 Java 对应类型。
#[derive(Clone, Debug)]
pub struct ContractCompilation {
    pub(crate) diagnostics: Vec<CompilationDiagnostic>,
    pub(crate) rules: Vec<RuleDefinition>,
    pub(crate) obligations: Vec<ContractObligation>,
    pub(crate) compiled: Option<CompiledContract>,
}
impl ContractCompilation {
    /// 返回准备状态，不输出allow或工程通过。
    pub fn status(&self) -> CompilationStatus {
        if self.compiled.is_some() {
            CompilationStatus::ReadyForPlanning
        } else {
            CompilationStatus::Blocked
        }
    }
    /// 返回全部阻塞诊断。
    pub fn diagnostics(&self) -> &[CompilationDiagnostic] {
        &self.diagnostics
    }
    /// 返回优先保留组织基线的有效规则。
    pub fn rules(&self) -> &[RuleDefinition] {
        &self.rules
    }
    /// 返回已读取的冻结义务。
    pub fn obligations(&self) -> &[ContractObligation] {
        &self.obligations
    }
    /// 只有全部准备条件满足才提供不可变编译产物。
    pub fn compiled(&self) -> Option<&CompiledContract> {
        self.compiled.as_ref()
    }
}
