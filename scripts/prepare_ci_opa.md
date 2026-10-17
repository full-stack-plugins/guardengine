# 固定 OPA 的 GitHub Actions 准备入口

`python -B scripts/prepare_ci_opa.py --output <绝对路径>` 只用于显式 GitHub Actions 运行：CI=true、GITHUB_ACTIONS=true、数字 GITHUB_RUN_ID、绝对 RUNNER_TEMP 均为前提。输出的父目录必须已存在，且必须位于 RUNNER_TEMP 的严格后代；拒绝符号链接、Windows reparse 路径、路径逃逸、特殊文件及已有不匹配原件。这里的 CI 环境限制是防止意外本地自动安装的操作约束，不是不可伪造的宿主身份或 K09 授信证明。

支持固定官方 `v1.21.2` Windows/amd64 与 Linux/amd64 static 两资产，名称、大小、SHA256 在源码中固定并由独立官方 release 核对。仅接受 GitHub 官方发布资产的 HTTPS 域名和重定向；先读取官方 `.sha256` 并与固定 pin 比对，再下载有界原件。资产大小和摘要完全一致后才以 create-if-absent 硬链接发布，不能覆盖另一进程的目标文件。已有正确原件仍须核对官方 checksum，并重算实际大小/摘要后才复用；不匹配则拒绝。

下载暂存文件在目标父目录，失败即删除本次暂存文件，不删除或覆盖其他所有者的目标；发布后移除暂存链接。设置执行权限与真实 `version` 探针发生在摘要验证之后。探针要求固定版本、OS/架构和 Rego v1；版本文本不是摘要来源。脚本不修改 PATH，不写全局目录、不安装 CLI/插件、不提供本地下载 fallback。普通环境直接退出2，不访问网络。

成功 stdout 只有闭合 metadata JSON：path、version、asset、size、sha256；失败诊断只写 stderr 并退出2。调用方在 job 的步骤里创建目录、读取 RUNNER_TEMP 并写入 GITHUB_ENV，不在 job.env 中使用 runner context。跨仓流程必须先发布 GuardEngine 准备器，再启用依赖它的 FlowGuard 工作流；缺失脚本应失败而不是降级下载。

`python -B scripts/tests/test_prepare_ci_opa.py` 使用受控 HTTP 字节流和显式测试 pin，验证边界、校验顺序、原子不覆盖与失败清理；这些字节绝非真实 OPA，测试将真实执行探针替换为受控回调。真实平台原件及运行仍须在 Linux/Windows Actions 执行，不能把本机受控15项测试宣称双平台已验。现有本机官方 OPA 不由本脚本重复下载。

不是 OS 沙箱：同一 OS 身份的恶意并发修改目录或已校验可执行文件不能单凭路径检查消除。受保护临时目录与独立 CI 控制面必须限制其他写入者。脚本不签发批准、策略结论或 ExecutionGrant。

GuardEngine 的 `kernel-components` 为 guard-policy 执行默认接口/包/异常解析测试；独立 `real-opa` 双 OS job 在工具和 FlowGuard 包准备完成后，显式启用 real-opa-tests，执行真实 OPA、注册计划、恢复及独立表差分。两类测试同时保留，没有用默认 feature 不执行真实工具来替代真实验收。

首次跨仓发布顺序：先提交发布 GuardEngine 的准备器和执行器，再发布 FlowGuard 的政策包及独立 cases，然后重跑先前依赖缺失的 GuardEngine real-opa job。第一次 core CI 若因尚未发布的 Flow 包失败，应保留其失败记录；不能把该次运行说成全绿、设 continue-on-error、跳过缺工具/缺包，或下载未核验替代品。后续两仓都在 main 时仍由双 OS job 每次读取真实依赖并验证。
