# Guard 公共工程方法库

此目录是第一批六个适配技能的唯一维护源：五项 ArchGuard 方法与 guard-grilling 依赖。来源固定 Matt Pocock skills 1.3.1 / `49dd158d1076134a641b33efb035946536778336`。

- `sources/`：固定提交的原始 Markdown、辅助文件与 MIT LICENSE，仅溯源，不作为宿主自动执行的技能。
- `adapters/`：适配后的中文技能，遵循原生规格、授权和 Guard 能力边界；MIT 许可保留 Matt Pocock 署名。
- `references/`：共享协议源码，构建时放入每个技能自己的 references，避免跨插件依赖假设。
- `catalog.json`：维护者声明的技能目录、直接依赖与参考文件映射。
- `skills.lock.json`：完整提交和所有来源/适配文件 SHA-256；构建只校验，不更新。

更新来源必须固定新提交，重新审查差异、许可和依赖，再显式更新锁。不能只改生成的 ArchGuard 插件文件，也不能将完整上游仓库中未发布的实验技能自动加入。

锁定源与生成包必须使用 LF 换行，由两个仓库的 `.gitattributes` 固定；锁命令拒绝 CRLF 工作树，避免 Windows 本地摘要与提交后 Linux/Windows 检出的字节不同。摘要校验始终比较原始字节，不会把漂移归一化后放行。

```text
python -B scripts/lock_skill_library.py --library skill-library
python -B scripts/skill_bundle.py --library skill-library --plugin ../archguard/plugin
python -B scripts/skill_bundle.py --library skill-library --plugin ../archguard/plugin --check
python -B -m unittest discover -s scripts/tests -p test_skill_bundle.py
```

后续插件通过自己的 `skills.request.json` 选择入口；构建器按依赖闭包打包，不安装宿主，不联网。版本/摘要校验不等于运行正确或受信签名。现有 architecture-check 是 ArchGuard 自有技能，不由本库覆盖。

生成记录位于插件 `org.partme.guard/skills.generated.json`。人工修改受管输出、未受管目录冲突、失效引用或摘要漂移会拒绝生成；需要先把有意修改迁回公共源，再协调受管输出。没有 --force。预检查失败不写输出；普通文件 I/O 失败回滚已修改文件。并发恶意替换不属于文件级生成器的隔离保证，应在受信固定工作区构建。
