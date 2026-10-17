# EG-K09 独立批准与生产者来源核验

2026-10-17。事实源为CodeGuard现有add-engineering-guard-core的EG-K09及EG-GATE-002/003。本页记录验收候选，完整新回归与双OS CI通过前不勾任务。

## 已发布批准核验

GuardEngine afa10825052e70f4918c6d28bf42a89579f1d313提供ApprovalVerifier和ReceiptApprovalAuthority；[CI38992692068](https://github.com/full-stack-plugins/guardengine/actions/runs/38992692068)21/21成功。文档提交58b8b69a810b6af091a312f5b9761fb06b2e0764的[CI38993606579](https://github.com/full-stack-plugins/guardengine/actions/runs/38993606579)也21/21成功。这些结果不覆盖尚未发布的来源增量。

批准签名使用独立域、原始wire JCS和Ed25519 strict，仅移除signature；闭合材料绑定受保护身份、意图、audience、实际三层契约及完整范围。逐次重查时钟、撤销和原件，不把解析的JSON当批准。实际迁移CAS前二验，拒绝同ID换有效签名、撤销后复活、范围扩大和候选错配；范围不变的批准在两个真实候选之间复用，不复用旧技术结论。

## 新来源增量

公开Ledger.start/finalize确实能写出可查询的当前PASS元数据，因此账本快照不证明真实执行。新增DispatchCompletion只有实际Runtime创建进程且终结Applied的内部路径可构造；无公开构造、Deserialize或from_store。Existing、未派发及AuditOnly均不能取得它。旧execute/v2和旧观察wire保持。

独立ProducerRegistry精确核对完整生产者、domain、verifier pin、契约/策略/范围、profile、argv与环境。ProducerSigningRequest仅从完成凭据派生机械事实，不接收任意GuardResult.assessment。签名固定独立来源域及原始wire，消费端重新核对来源权威、期限/撤销、当前计划与完整运行头、序号/operation、原始metadata和双流摘要尺寸。跨重启必须有独立签名，不能从裸数据库恢复完成凭据。

该来源profile覆盖Runtime的domain verifier。PolicyEngine不借用domain身份；OPA继续由K08独立Bound执行器实际核对固定制品/版本/包/计划，观察保持UNVERIFIED。本页不宣称OPA、CodeGraph和CodeReview等角色已具备统一外部可携带签名证明。

真实失败、超时或部分输出只保留对应来源，不转成SATISFIED/COMPLETE；零退出同样不证明质量。新增assessment字段被闭合原件拒绝。签发期限不刷新真实运行时间；本切片不声明外部数据库TTL或全局来源maxAge。

## 实际增量验证

- 作者先真实RED：已运行exit7进程，公开账本写伪PASS可查询，而内部完成凭据尚缺失；实现后验证来源与元数据分离。
- 另一真实竞争反例取得Existing票提前写伪PASS，真实Runtime只能AuditOnly且无完成凭据。
- 来源组合E2E先RED后GREEN；夹具互斥锁自锁被终止的中间运行不算通过。
- 根在作者停写后的最终source测试程序独立执行：1/1通过，64.26秒，零ignored。实际覆盖exit7与独立ring签名、跨重启验证、有效重签错身份/profile/argv/env、弱key、注入assessment、同ID替换、新伪头使旧签名过期、原件损坏持久隔离、撤销不复活及到期；按一个组合用例计数，不把断言数量算成测试数。
- 授权端新增OperationAuthorization真实签名正例及换subject/错action反例。同级独立运行最终verification程序15/15通过，0.73秒，零ignored。

作者最终锁定依赖离线串行完整回归已通过：Runtime 4单元、14真实E2E（575.70秒）及2编译失败文档测试，共20；Ledger原10个reported（3.95秒，含1个worker辅助入口）；Authority 7真实Git/SQLite（97.01秒）、1来源组合（62.57秒）、15签名边界（0.70秒）、3编译失败文档测试，共26。均零ignored。三个crate的all-features/all-targets严格Clippy、格式和diff检查通过。

新来源提交的双OS CI仍待终态；当前EG-K09保持未勾选，已发布旧提交绿灯不替代本批验证。

## 任务边界

K09验收独立核验端口和真实签名提供者；受保护注册、撤销、原件与时钟是明确宿主前提，测试内存端口不证明生产部署隔离。专业raw解析与Flow组合归L02，grant签发/消费及控制端隔离归K10，组织身份服务另按后续任务交付。不用这些后续能力倒推本切片已具备生产强制执行，也不把它们扩成K09原任务以外的前置。
