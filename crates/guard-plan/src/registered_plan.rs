use crate::{FrozenPlan, PlanningOutcome};
use guard_protocol::TaskBinding;
use guard_store::{EvidenceStore, HeadIdentity};
use std::path::Path;
/// 经私有证据原件及 active head CAS 注册的绑定；不支持 Deserialize，不产生执行授权。
/// 使用前仍须 assert_current，不能把此类型当作 K09/K10 可信 grant。无 Java 对应类型。
#[derive(Clone, Debug)]
pub struct RegisteredPlan {
    pub(crate) outcome: PlanningOutcome,
    pub(crate) head_key: String,
    pub(crate) head: HeadIdentity,
}
impl RegisteredPlan {
    /// 返回不可变捕获原件供受控执行准备读取；复制它不产生注册计划或执行资格。
    pub fn captured_subject(&self) -> &guard_subject::CapturedSubject {
        &self.outcome.subject
    }
    /// 返回冻结的捕获绑定声明；不证明真实 OS、DLL 或工具链环境已被独立验证。
    pub fn capture_binding(&self) -> &guard_subject::CaptureBinding {
        &self.outcome.subject.binding
    }
    /// 返回已注册绑定下的计划，只在 assert_current 成功的当前对象上使用。
    pub fn plan(&self) -> &FrozenPlan {
        self.outcome
            .plan
            .as_ref()
            .expect("registration requires complete plan")
    }
    /// 返回 active binding 的全部范围与授权引用；不允许修改。
    pub fn binding(&self) -> &TaskBinding {
        &self.outcome.binding
    }
    /// 返回只读 head 身份，用于审计关联，复制它不能制造 RegisteredPlan。
    pub fn head(&self) -> &HeadIdentity {
        &self.head
    }
    /// 消费前重核 active baseline 身份、未过期/完整原件及实际 Git 当前候选。
    /// 返回成功只证明绑定和输入仍匹配，不证明政策内容/运行环境或授权来源可信。
    pub fn assert_current(
        &self,
        store: &EvidenceStore,
        project: &Path,
        now_ms: u64,
    ) -> Result<(), String> {
        self.assert_binding_current(store, now_ms)?;
        if project.canonicalize().map_err(|e| e.to_string())? != self.outcome.project {
            return Err("registered worktree root differs".into());
        }
        let subject = &self.outcome.subject;
        guard_subject::verify_current(project, &subject.binding, &subject.selection, subject)
    }
    pub(crate) fn assert_binding_current(
        &self,
        store: &EvidenceStore,
        now_ms: u64,
    ) -> Result<(), String> {
        let head = store
            .read_head(&self.head_key, now_ms)?
            .ok_or("active task binding missing")?;
        if head.identity != self.head
            || head.bytes != crate::registration::baseline_bytes(&self.outcome)?
        {
            return Err("registered task binding is no longer current".into());
        }
        Ok(())
    }
}
