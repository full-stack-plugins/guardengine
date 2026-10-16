# 专业 Guard 并行实施批次

用户要求创建多个智能体，以 CodeGuard 的成熟度为参照，加速其他 Guard 专业能力落地。本 change 是此次跨仓库实施的唯一规格事实源；不初始化各子仓库的第二套规格。

本批由独立智能体负责 GitGuard、SpecGuard、TestGuard 和 FlowGuard，主代理负责共享接入、ArchGuard 兼容回归、交叉审查和验证证据汇总。各仓库已有未提交修改保留，各工具的报告不能越权替代其他工具的证据。

增量：GitFlow 激活策略的只读检查；原生规格结构及漂移复验；JUnit/Surefire 报告一致性核验；既有十阶段 FlowGuard 的安全只读接入。CodeGuard 不新增检测器，ArchGuard 保留此前 0.5.0 Java/Maven/Cargo 检查与模型闭环。

本批不承诺消除所有成熟度差距；可信测试执行器、完整编译器、远程批准/发布、自动 Git 写入不在范围内。
