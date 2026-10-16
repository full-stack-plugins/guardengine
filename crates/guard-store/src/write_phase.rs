/// 持久化进度通知；观察者不能覆盖数据库结果或授予信任。无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WritePhase {
    /// pending 意图及审计已经提交。
    IntentDurable,
    /// 原件已经 sync，complete 事务尚未提交。
    ArtifactDurable,
    /// complete 与对应审计事务已提交。
    Complete,
}
