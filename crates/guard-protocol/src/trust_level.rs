/// 消费端记录的信任维度，候选 JSON 中没有可自报的升级字段。无 Java 对应类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrustLevel {
    /// 未经独立来源和签名核实。
    Unverified,
    /// 协作模式，仍不能冒充服务端强制。
    Cooperative,
    /// 仅由未来独立消费控制端验证产生，本 crate 的 ingress 永不返回此值。
    Verified,
}
