/// 内部调用档位与附属凭据选择，旧API不执行新来源资格读取。无Java对应类型。
#[derive(Clone, Copy)]
pub(crate) enum DispatchProfile {
    LegacyV1,
    ObservedV1,
    LegacyV2,
    ObservedV2,
}
impl DispatchProfile {
    pub(crate) fn v2(self) -> bool {
        matches!(self, Self::LegacyV2 | Self::ObservedV2)
    }
    pub(crate) fn receipt(self) -> bool {
        matches!(self, Self::ObservedV1 | Self::ObservedV2)
    }
}
