## ADDED Requirements

### Requirement: 可复现的技能依赖包
公共构建器 MUST 从固定提交和摘要生成 ArchGuard 五项技能及依赖；缺失、循环、摘要漂移、越界路径或受管文件人工修改 MUST 拒绝，且不得部分覆盖输出。
#### Scenario: 离线构建与漂移
- **WHEN** 已锁定技能来源和适配文件生成包后再次 check
- **THEN** 输出一致且不联网；篡改任一受管输入或输出后 check 失败。

### Requirement: 单一原生规格来源
共享适配器 MUST 只引用已有规格；未初始化或存在歧义 MUST 明确报告，不自动创建或迁移体系。
#### Scenario: 多体系共存
- **WHEN** 项目同时存在 OpenSpec 与 Spec Kit
- **THEN** 自动发现返回 ambiguous；显式 system 与 target 可选择现有目标，返回实际文件及摘要。

### Requirement: 架构规则追踪与证据复验
ArchGuard MUST 支持可选原生规格绑定，完整检查规则到规格的引用，规格变化 MUST 使已保存报告失效。绑定不得充当用户授权。
#### Scenario: 规格或源码变更
- **WHEN** Maven 违规修复后重新检查，并保存新报告
- **THEN** 新报告可 verify，修复前报告失效；随后修改绑定规格，修复后报告也失效。

### Requirement: 有界 Maven profile 场景
分析器 MUST 对每个影响架构模型的 profile 要求明确选择，合并选中模型并将场景配置纳入证据；未知或未选择配置 MUST 阻断。
#### Scenario: profile 引入违规依赖
- **WHEN** 显式启用包含越层依赖的 profile
- **THEN** 报告违规源位置；移除该依赖后检查通过，旧报告不能通过复验。

### Requirement: Cargo 与接入兼容
现有 Cargo 分析和六 Guard 安装/MCP 入口 MUST 保持兼容。方法建议 MUST 与确定性检查、用户审批分别呈现。
#### Scenario: 未绑定规格的旧项目
- **WHEN** 旧 Cargo 项目执行原有 check 和 verify
- **THEN** 保持原有判定与退出码，不因缺少新增规格配置失败。
