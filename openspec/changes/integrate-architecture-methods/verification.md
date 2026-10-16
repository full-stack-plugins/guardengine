# 第一批整合验证记录

日期：2026-10-16（Asia/Shanghai）。环境：本地 Windows/PowerShell；构建缓存、测试项目和临时文件均在 E:/workspaces/workspace-agent-skills/.guard-build。没有安装到真实宿主，没有改动业务项目，没有提交或发布。

## 对应实现

- 公共方法来源：skill-library/catalog.json 与 skills.lock.json，Matt Pocock 1.3.1 / 49dd158d1076134a641b33efb035946536778336；25 个来源/适配输入摘要，保留 MIT 许可。
- 自包含包：ArchGuard 0.3.0 plugin，五项 archguard-* 方法 + guard-grilling 依赖 + 原有 architecture-check；包内引用检查与受管输出冲突保护。
- 原生规格：guard-install spec context 及 context.specification；OpenSpec、Spec Kit、已有 Superpowers；多体系/目标显式选择；不初始化，不推断批准。
- 规则追踪：ArchGuard .archguard/spec-binding.json；精确规则集合、原生产物与完整需求标识校验；源/契约/绑定/规格变化使旧报告失效。
- Maven：.archguard/maven.json 显式 true/false 场景；选中 profile 属性、模块与依赖合并，未选、未知 ID、重复 XML ID、错误类型阻断。Cargo 保持原有入口，并新增 Spec Kit 绑定复验用例。

## TDD 证据

实现前真实执行新增测试：

1. context_discovers_native_spec_without_creating_files：specification 为 Null，而不是 unconfigured。
2. 两个原生规格选择测试：spec context 命令尚不存在，子进程失败。
3. selected_profile_violation_repair_and_configuration_drift：模型 profile 只返回 INDETERMINATE，无法给出真实违规 FAIL。
4. native_spec_binding_is_validated_and_invalidates_old_reports：修改原生规格后 verify 仍返回 0，预期 4。
5. 技能打包的两个正向用例：缺少构建器，不能生成依赖闭包；错误分支随后增加 JSON 诊断断言，避免把解释器找不到文件当作预期校验失败。

以上失败后补充实现，全部转绿，并扩展负例和回归。

## 执行结果

| 命令（以 guardengine 为工作目录） | 结果 |
|---|---|
| cargo test --manifest-path ../archguard/Cargo.toml --locked --all-targets | 23 passed：architecture_loop 18 + Cargo integration 5 |
| cargo test --manifest-path crates/guard-install/Cargo.toml --locked --features test-fixture | 8 passed：内部回滚 1 + 安装 2 + 原生规格 5 |
| cargo test --locked --all-targets | 19 passed：CLI evidence 2 + contract 8 + graph 9 |
| python -B -m unittest discover -s scripts/tests -p test_skill_bundle.py | 5 passed |
| python -B scripts/verify_guard_installation.py --bin-dir <E盘构建目录>/debug --gitflow-root ../gitflow-plugin | 9 passed，覆盖六工具真实二进制 |
| python -B scripts/skill_bundle.py --library skill-library --plugin ../archguard/plugin --check | verified，6 个生成技能，changed_files=[] |
| cargo clippy（ArchGuard all-targets；guard-install all-targets + test-fixture）-- -D warnings | 通过 |
| cargo fmt（ArchGuard 与 guard-install）-- --check | 通过 |
| openspec validate integrate-architecture-methods --strict | 通过 |
| git diff --check（guardengine / archguard） | 通过 |

合计 64 个已执行测试用例，重复运行不重复计数。零测试的 lib/main/doc harness 不计入。CodeGuard 使用 --no-default-features 的接入构建；没有因此宣称其完整专业检测器通过。

## 需求到验证

- 可复现依赖：闭包生成、重复构建不变、缺失依赖、循环、锁摘要漂移、人工修改、未受管目录冲突、缺失 Markdown 引用、路径越界、空请求。
- 原生规格：无初始化行为、OpenSpec 目标/文件摘要、双体系显式选择、Spec Kit constitution、Superpowers 目标、多目标歧义、缺文件、排除归档、越界目标、文件大小和目录预算。
- Java/Maven：profile 引入违规→explain/check→实际修改构建声明→新 check/verify；场景变更使旧报告失效；profile 模块、管理依赖和属性继承；同 POM 后声明优先；XML 重复 ID 拒绝。
- 规格证据：原生规格变更使旧报告失效，规则映射遗漏和 REQ-1/REQ-10 子串混淆拒绝；绑定参考只能指向当前原生目标。
- MCP：真实 stdio initialize/tools/list/tools/call；已绑定规格及显式 profile 项目与 CLI 报告相同，双体系下 context 显式选择有效；非法路径/导出参数拒绝。
- Cargo：旧 v1alpha1 正负例保留；新增 Cargo + Spec Kit 绑定、当前报告验证及 plan 漂移拒绝。
- 安装：六工具共存、用户/项目范围、三宿主配置、卸载、人工冲突、只读 MCP、动态 GitFlow、Windows 链接保护。

## 审查与限制

已核对缺覆盖不放行、唯一原生规格、依赖闭包、受管文件冲突、路径和输入预算、许可与手动调用限制、CLI/MCP 共用实现、版本变化和旧报告兼容性。添加 LF checkout 约束，避免跨平台换行破坏技能锁摘要。

ready 仅是文件就绪，不替代原生规格语义校验或用户批准。绑定验证标识存在，不证明自然语言与规则语义等价。旧项目不配置绑定时保持原行为；0.3.0 分析器版本变化要求重新生成报告。

Maven 仅显式静态场景，不执行 Maven，不下载外部父/BOM，不求值环境/settings/扩展激活；Java import/类型/方法不在覆盖中。不宣称 ddd4j 全项目已经通过，也没有替该项目选定 profile。

Skill 包已离线检查；MCP 通过实际 CLI 子进程验收，但真实 Codex/Claude/Gemini 插件加载和远端 Linux/Windows CI 尚未运行。宿主配置安装不自动安装五项技能；需要通过宿主支持的插件加载机制使用包。自动化测试不等于 LLM 访谈或架构判断效果评测。

报告未签名；本地文件摘要不是可信授权。普通写入失败回滚不是操作系统级隔离，不承诺恶意并发文件替换下的安全事务。远端跨仓 CI 需要协调发布匹配的 guardengine/archguard 代码，不能假设默认分支已包含未提交改动。

## 规格状态

第一批实现与本地验证对应本 change，未迁移或改写此前变更的验收。保留活动 change 供评审，未归档；后续先补完整有效模型与 Java 源级检查，再进入第二批任务实施/TDD/评审编排。两份补存目标设计及技能方案的入口见 docs/guard-design-index.md。
