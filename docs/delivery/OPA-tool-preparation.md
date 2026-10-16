# EG-K08：真实 OPA 工具准备

核对日期：2026-10-17。本机 `Get-Command opa` 未发现 CLI；尚未下载或执行。此页记录待授权的本地测试依赖，不代表策略引擎或 K08 已完成，也不执行全局安装。

## 已核对的官方来源

- [OPA 安装说明](https://www.openpolicyagent.org/docs)：官方二进制及追加 `.sha256` 的校验文件。
- [不可变版本 v1.21.2](https://github.com/open-policy-agent/opa/releases/tag/v1.21.2)，发布时间 2026-10-12。
- [Windows amd64 原件](https://github.com/open-policy-agent/opa/releases/download/v1.21.2/opa_windows_amd64.exe) 与[官方摘要](https://github.com/open-policy-agent/opa/releases/download/v1.21.2/opa_windows_amd64.exe.sha256)。GitHub release API 同时给出同一个制品摘要。

固定大小：64,622,592 字节。固定 SHA256：`0089fa7ad8cdea7ff9e56ff86fa9202462afb63ef74506e2b547a24b4a987ccb`。

## 授权后执行的精确范围

只写入 `E:/workspaces/workspace-agent-skills/.guard-build/tools/opa/1.21.2/`。不改全局 PATH、不初始化受治理项目、不覆盖其他 OPA；已有文件必须与固定摘要一致，否则停止。

```powershell
$opaToolRoot = 'E:/workspaces/workspace-agent-skills/.guard-build/tools/opa/1.21.2'
$opaToolFile = Join-Path $opaToolRoot 'opa.exe'
$opaExpected = '0089fa7ad8cdea7ff9e56ff86fa9202462afb63ef74506e2b547a24b4a987ccb'
if (Test-Path -LiteralPath $opaToolFile) {
    if ((Get-FileHash -LiteralPath $opaToolFile -Algorithm SHA256).Hash.ToLowerInvariant() -ne $opaExpected) {
        throw 'Existing OPA does not match the approved pin; refusing to overwrite.'
    }
} else {
    New-Item -ItemType Directory -Path $opaToolRoot -Force | Out-Null
    $opaDownload = Join-Path $opaToolRoot ('opa.download.' + [guid]::NewGuid().ToString('N'))
    Invoke-WebRequest -Uri 'https://github.com/open-policy-agent/opa/releases/download/v1.21.2/opa_windows_amd64.exe' -OutFile $opaDownload
    if ((Get-Item -LiteralPath $opaDownload).Length -ne 64622592 -or
        (Get-FileHash -LiteralPath $opaDownload -Algorithm SHA256).Hash.ToLowerInvariant() -ne $opaExpected) {
        throw 'OPA download length/digest mismatch; do not execute this file.'
    }
    Move-Item -LiteralPath $opaDownload -Destination $opaToolFile
}
& $opaToolFile version
if ($LASTEXITCODE -ne 0) { throw 'OPA version verification failed.' }
```

下载授权正在单独询问。批准前不执行以上片段；代码、接线及不依赖该工具的检查继续推进。运行测试时还必须实际核验版本、制品 pin、策略包和受限能力，不能仅凭这里的摘要文字信任工具。其他平台制品应分别核对并验收，不复用 Windows 摘要。
