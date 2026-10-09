//! Borrowed-field construction prevents projection fanout before evaluation budgets run.
use super::{IntegrationError, MAX_ARTIFACT_BYTES, adapter::bounded_json_size, fail};

/// Integration-profile builder. Checks each borrowed fact before copying any fields.
/// At most 4096 facts and 16 MiB combined JSON/owned-field estimates are accepted.
/// Any rejected insertion poisons the builder: partial facts cannot be returned as success.
#[derive(Default)]
pub struct FactBudget {
    facts: Vec<crate::GuardFact>,
    charged: usize,
    rejected: bool,
}
impl FactBudget {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn push_relation(
        &mut self,
        subject: &str,
        predicate: &str,
        object: &str,
        source: &str,
    ) -> Result<(), IntegrationError> {
        if self.rejected {
            return fail("fact construction budget already rejected");
        }
        self.rejected = true;
        #[derive(serde::Serialize)]
        struct BorrowedFact<'a> {
            subject: &'a str,
            predicate: &'a str,
            object: &'a str,
            source: &'a str,
        }
        let borrowed = BorrowedFact {
            subject,
            predicate,
            object,
            source,
        };
        let serialized = bounded_json_size(&borrowed)?;
        // Count both encoded bytes and owned payload, plus conservative Vec spare capacity.
        let cost = serialized
            .saturating_add(subject.len())
            .saturating_add(predicate.len())
            .saturating_add(object.len())
            .saturating_add(source.len())
            .saturating_add(2 * std::mem::size_of::<crate::GuardFact>())
            .saturating_add(1);
        let total = self.charged.saturating_add(cost);
        if self.facts.len() >= 4096 || total > MAX_ARTIFACT_BYTES.saturating_sub(2) {
            return fail("fact construction budget exceeded");
        }
        self.facts
            .try_reserve(1)
            .map_err(|_| IntegrationError("fact allocation failed".into()))?;
        self.facts.push(crate::GuardFact {
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
            source: source.into(),
        });
        self.charged = total;
        self.rejected = false;
        Ok(())
    }
    pub fn finish(self) -> Result<Vec<crate::GuardFact>, IntegrationError> {
        if self.rejected {
            return fail("fact construction budget rejected; incomplete projection discarded");
        }
        Ok(self.facts)
    }
}
