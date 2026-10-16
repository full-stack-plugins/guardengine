# 设计

公共库位于 guardengine/skill-library，原始来源与适配后的技能分离。锁文件记录完整提交及每个输入 SHA-256。Python 标准库构建器解析显式依赖闭包，验证路径和摘要，生成 skills/<name>/SKILL.md、所需参考资料和来源许可；构建前验证全部受管输出，拒绝覆盖人工修改。离线 check 不联网。

guard-install 提供 spec context 及 context 中的 specification 投影。支持 system=openspec|speckit|superpowers 和目标相对路径；多体系或多目标返回 ambiguous，缺产物返回 incomplete。输出原生文件路径、缺失项、摘要，不创建第二套规格，不表示用户已批准。所有扫描有界、拒绝链接逃逸；归档 change 不自动选择。

ArchGuard 使用可选 .archguard/spec-binding.json 声明 system、target 和 rules 映射，每条规则关联原生 artifact 与 requirement 文本标识。绑定只证明引用存在和版本一致，不判断需求语义或授权真实性。绑定存在时，规则映射必须完整，引用必须属于已选原生目标，报告快照包含绑定和原生规格摘要。未绑定旧项目保持行为兼容。

Maven .archguard/maven.json 显式声明 profiles: {id: boolean}。这是静态分析场景，不宣称自动求值 Maven 的 JDK/OS/settings/environment 激活。任何影响模型的 profile 未选择、未知 ID、重复 ID 均产生 partial。选中 profile 的属性、依赖、管理依赖及模块按文档顺序合并；配置本身纳入输入摘要。外部父/BOM、复杂有效模型保持 fail closed。

技能以中文指导已有源码事实、术语/ADR、需求追踪和规则五种断言；不把深模块等启发式判断当作硬门禁。仅通过现有检查实现调用 CLI/MCP。运行研究无需强制子 Agent，不操作远端、不自行创建分支或安装依赖。
