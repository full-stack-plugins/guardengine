# 共享证据契约和职责

TestGuard 实施真实执行器；SpecGuard 实施原生需求、源码声明映射和执行用例追踪；FlowGuard 实施显式公钥、计划、候选和时效核验。主代理维护 guard-evidence crate、跨仓库集成与审查。

## Source snapshot

`snapshot_project(root)` 返回 schema_version=guard.source-snapshot/v1、digest（sha256:前缀）、files=[{path:项目相对路径,sha256:sha256前缀}]、scope。确定性遍历全部普通文件，排除任意层的 .git、target、node_modules、.guard-build；这些排除必须出现在 scope。拒绝链接/项目外、读取错误、单文件>4MiB、文件>8192、目录>16384、总内容>128MiB、深度>64。不是完整 Git 提交或编译器有效输入证明，环境/外部运行时依赖另行记录。执行输出必须项目外，源码前后快照一致才允许 PASS。

## Signed receipt

```json
{
  "schema_version":"guard.signed-evidence/v1",
  "payload":{
    "schema_version":"testguard.execution/v1",
    "producer":"testguard",
    "task_id":"REQ-001",
    "project":"规范项目绝对路径",
    "snapshot":{"schema_version":"guard.source-snapshot/v1","digest":"sha256:...","files":[],"scope":{}},
    "plan_digest":"sha256:...",
    "runner":"maven",
    "executable":{"path":"绝对路径","sha256":"sha256:..."},
    "argv":[],
    "started_at_unix_ms":0,
    "finished_at_unix_ms":1,
    "exit_code":0,
    "status":"PASS",
    "required_tests":["Example#accepts"],
    "observed_tests":["Example#accepts"],
    "artifacts":[{"path":"项目外绝对路径","sha256":"sha256:..."}],
    "execution_verified":true
  },
  "signature":{"algorithm":"Ed25519","public_key":"64位小写hex","value":"128位小写hex"}
}
```

签名输入是 UTF-8 `guard.signed-evidence/v1\n` 加递归键排序、数组保序的紧凑 JSON payload。digest_json 使用相同 JSON 规范化并返回 sha256:前缀；hash_file 同样返回该前缀。签名密钥是 PKCS#8 原始字节，公钥是含32字节公钥的小写hex文本。`keygen` 只能显式创建新文件，不覆盖；密钥/受信公钥必须由调用者在项目外选择。验签仅证明指定密钥签署了 payload，不验证 payload 业务含义。

执行器可输出未签名回执，但此时不得称为受信。各消费者不能从回执自选公钥，必须显式固定公钥/计划义务、验证源码/产物当前字节和时效、核验 PASS/exit0/必需集合/运行器/生产者等一致性。导入一致性不能伪装执行证明。测试代码与同用户密钥并非 OS 隔离；生产受信前提是受控 CI/生产者保护密钥、执行程序和工作区，不能宣称本地签名已提供远程证明或沙箱。

## Execution and trace

真实执行仅由 CLI --execute 触发，MCP 保持只读。运行器采用受限预定义 Maven/Cargo 命令，冻结计划不接收任意 shell；超时、进程失败、零/跳过测试、缺必需项、源码漂移、旧报告不能 PASS。过程日志和真实报告必须保留，修复后重新执行产生新回执，verify 重算当前计划、源码与产物。

执行计划版本 `testguard.execution-plan/v1` 必填 task_id、runner、required_tests、timeout_seconds；task_id 原样写入签名 payload。FlowGuard 的外部 policy 精确绑定 task_id、plan_digest、runner、required_tests 和 max_age_seconds，不能用调用者临时标签替代签名任务绑定。Windows Maven 从显式 Maven 分发目录启动 Java classworlds，记录相关程序与启动文件指纹；不经 cmd 拼接命令。首版不接受已有 Surefire 报告或 `.mvn` 启动参数文件，需在隔离干净候选中运行。

追踪 manifest 引用真实原生 artifact/id、项目内源码声明和必需测试 ID。文件/标记存在只证明声明映射可定位，不证明语义实现；完整通过还需匹配当前源码的受信 PASS 执行回执。FlowGuard 证据齐备最多 requires_approval，不推进阶段。
