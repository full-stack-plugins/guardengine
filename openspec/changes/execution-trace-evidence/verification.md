# 验证记录（2026-10-16）

## 实现与证据

- 共享 `guard-evidence`：源码快照、显式 Ed25519 密钥、签名/当前执行验证，5 项测试和 Clippy 通过；CI 已加入独立 crate 检查。
- TestGuard：保留 29 项导入 JUnit 用例；真实 Maven/Cargo 执行、task_id 计划绑定、签名回执及只读 MCP。最终 37 项常规测试通过；额外 opt-in 真实 Maven 双模块 reactor 测试实际执行通过（112.30 秒），验证失败、修复、新报告和旧报告拒绝。
- SpecGuard：原 22 项检查加 12 项追踪检查，34 项通过；原生需求 artifact/id、源码声明及必需测试关联，签名篡改、旧源码/产物和伪旧报告均阻断。
- FlowGuard：5 项证据测试和原 4 项阶段测试通过；外部公钥、任务、计划、runner、测试集合、有效期和当前产物检查；技术通过仍返回 requires_approval。
- 真实六工具安装回归 10/10 通过；五个专业插件包结构、技能引用和真实 stdio MCP discovery 通过：ArchGuard 10、GitGuard 3、FlowGuard 4、SpecGuard 8、TestGuard 4 项 MCP 工具。此项不等于宿主已经安装/加载插件。

## 跨组件真实联测

实际执行 `scripts/verify_execution_loop.py`，显式选择本机 Maven 3.9.9、JDK 17 和 Cargo；全部工作区在 E 盘隔离夹具。两种 runner 均完成：

1. TestGuard 真实执行并签名，verify 对照当前计划/源码/产物通过。
2. SpecGuard 原生需求→源码声明→受信执行测试关联 PASS，完整重算 verify 通过。
3. FlowGuard 技术检查通过，结果为 requires_approval。
4. 修改源码后同一回执被拒绝；恢复完全相同源码后重新核验通过。

Cargo 夹具还先产生实际失败回执，修复后执行新候选。Maven 单模块与双模块的真实失败修复另由 TestGuard 原生执行测试覆盖。

隔离结果目录：`E:/workspaces/workspace-agent-skills/.guard-build/testguard-real-20261016/execution-loop-f1b7414d2d954a0faef3ca6ed63a87d2`。其中私钥和运行产物只保留本地，未纳入 Git。

## 审查与边界

独立代理审查发现 Maven 报告在严格解析前可能无界读取，已增加单报告 4MiB、累计 64MiB、最多 1000 报告、XML 节点/深度和目录遍历预算；Cargo 摘要计数一致性已通过先红后绿负例修复，核对每套件与累计成功计数。最终二进制重新核验上述两种真实回执通过。正常失败、跳过、缺必需项、无签名、错误公钥、产物或源码变更不能提升为受信成功。

本地签名证明显式受信密钥签署了回执；同用户测试代码与私钥没有操作系统隔离。生产使用需要受保护的 CI 生产者、密钥、执行程序和候选工作区。源码快照按已声明目录排除和预算覆盖项目文件，不代表完整 Git 身份、外部工具链输入或业务语义正确；追踪声明也不替代语义验收。Windows Maven 首版不接受 `.mvn` 启动参数文件，已有 Surefire 报告需使用干净隔离候选处理，不自动删除用户产物。

CodeGuard 本批只复验共享集成构建和安装入口，未宣称重新验证其全部专业分析器。规格已通过 OpenSpec strict validate，保留增量供后续评审和归档。
