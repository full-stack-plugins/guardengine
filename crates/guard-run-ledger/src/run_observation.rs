use crate::RunState;
use guard_protocol::{Digest, RunSequence};
/// 针对调用方期望完整输入身份的本地观察。无Java对应类型。
#[derive(Clone, Debug)]
pub struct RunObservation {
    pub sequence: RunSequence,
    pub state: RunState,
    pub result_digest: Option<Digest>,
    pub trust: &'static str,
}
