use guard_contract::{CapabilityProbe, VerifierObservation, VerifierPin};
use guard_protocol::Identifier;
use std::collections::BTreeMap;
/// 来自实际固定argv探测的身份，编译期间不执行工具。无Java对应类型。
pub(crate) struct ProbeResults(pub BTreeMap<Identifier, Result<VerifierPin, String>>);
impl CapabilityProbe for ProbeResults {
    fn observe(&self, pin: &VerifierPin) -> VerifierObservation {
        match self.0.get(&pin.verifier_id) {
            Some(Ok(actual)) if actual == pin => VerifierObservation::Available(actual.clone()),
            _ => VerifierObservation::Unknown(
                "pinned version/artifact/config probe unavailable".into(),
            ),
        }
    }
}
