use crate::{CompilationDiagnostic, LayerLevel};
use guard_protocol::Identifier;
pub(crate) fn issue(
    items: &mut Vec<CompilationDiagnostic>,
    code: &str,
    layer: Option<LayerLevel>,
    rule: Option<&Identifier>,
    detail: &str,
) {
    if items.len() < 256 {
        items.push(CompilationDiagnostic {
            code: code.to_owned(),
            layer,
            rule_id: rule.cloned(),
            detail: detail
                .chars()
                .filter(|c| !c.is_control())
                .take(1024)
                .collect(),
        });
    }
}
