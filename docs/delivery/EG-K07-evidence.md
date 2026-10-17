# EG-K07 CodeGuard 公共投影与原生同次执行验收记录

2026-10-17；专业投影提交 `1f0e361fdebebe6c497a836f42de229fad83afa3`，当前候选 `640d2b9d`。原任务是 completion/findings/evaluate_delivery 对照、真实报告及旧协议双读，保持 local_unverified/not_evaluated。

适配器仅复用公共协议和既有原生解析器；专业诊断仍由 CodeGuard 产生。闭合历史 run_report/check_feedback 双读、重复键/未知字段/版本拒绝、原始字节保留、缺规则或源码证明保持 unbound、Partial 中已确认阳性保留。投影不签发批准或授权。

当前 P3C 普通项目0.3、任务复验0.4、任务预览0.40、文件反馈0.2各有独立闭合定义；旧Schema和历史原件保持。实际 classify、saved-event、next、attempt、sync 消费者共用同一原生产分类函数，不把新wire仅接入主检查而遗漏修复链。

Maven observer从原CLI同次进程捕获源码、原生报告与双流；成功回调及拒绝回调各执行本次检查，回调故障独立记录，不覆写旧退出行为。时间是实际调用观察边界，不伪称OS精确spawn；仍为UNVERIFIED。

已取得证据：

- 根独立Rust1.85 focused harness36/36通过（18.62秒）；作者同套36/36（22.47秒）、Clippy及格式通过。
- crate_boundaries 8/8已在D盘独立target执行；112份静态Schema闭包独立核验通过，无外部Schema检索。
- 交叉审查P3C严格身份/外层及Partial保留，未发现可确认阻断；额外2项P3C和3项Partial实际测试通过。
- [CI38989753140](https://github.com/full-stack-plugins/codeguard/actions/runs/38989753140)的早期真实CLI/P3C修复链、同次真实Maven、分层及MSRV步骤已经成功；该完整gate随后仅CSS有效格式夹具失败。4a82e922直接提供标准格式、移除吞掉结果的预格式化，保留真实格式检查与错误样例；[新CI38990883250](https://github.com/full-stack-plugins/codeguard/actions/runs/38990883250)已通过完整default suite及MSRV，WASM/npm/grammar后续仍待终态。不能用旧提交绿灯替代。

真实失败历史保留：旧0.73装配与Schema不一致由真实CLI暴露；P3C新增字段冒称0.2的坏原件冻结并持续拒绝；后续新增独立身份，而非放宽旧定义。254f Maven样例超出保守POM资格，修正共享测试输入；4ca9真实工具Exited0、捕获1次，但额外JDK21构造器文档诊断未预先映射，按生产边界正确unbound导致验收失败。当前仅补测试固定规则表，新负例证明移除映射仍unbound，未删原断言或降低生产限制。

状态：候选已发布，当前完整CI待终态，EG-K07保持未勾选。不是57语言或32grammar完整生产验收，也不代替K09授权、K10隔离、Q01完整产品接入。

后续38990883250终态失败：默认suite、MSRV、真实CLI与同次Maven、npm均成功，但WASM Go公共入口仍断言0.49/0.50，实际精确当前输出为0.73。640d2b9d仅修正该当前断言和另一CVE报告导出条件，不改历史Schema或兼容原件；格式通过，当前报告装配回归1/1实际通过18.62秒。[新完整CI38993532494](https://github.com/full-stack-plugins/codeguard/actions/runs/38993532494)仍须终态验收，不能从其他已过步骤推断此项通过。
