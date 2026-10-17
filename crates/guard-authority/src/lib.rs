//! EG-K09独立批准核验，不提供签发、执行授权或专业策略。
mod approved_scope;
pub use approved_scope::ApprovedScope;
mod approval_purpose;
pub use approval_purpose::ApprovalPurpose;
mod approval_layer;
pub use approval_layer::ApprovalLayer;
mod approval_material;
pub use approval_material::ApprovalMaterial;
mod principal_role;
pub use principal_role::PrincipalRole;
mod authority_anchor;
pub use authority_anchor::AuthorityAnchor;
mod anchor_source;
pub use anchor_source::AnchorSource;
mod authority_clock;
pub use authority_clock::AuthorityClock;
mod revocation_snapshot;
pub use revocation_snapshot::RevocationSnapshot;
mod revocation_source;
pub use revocation_source::RevocationSource;
mod receipt_bundle;
pub use receipt_bundle::ReceiptBundle;
mod receipt_source;
pub use receipt_source::ReceiptSource;
mod verification_request;
pub use verification_request::VerificationRequest;
mod verified_approval;
pub use verified_approval::VerifiedApproval;
mod signing_profile;
pub use signing_profile::{SIGNING_DOMAIN, signing_bytes, wire_signing_bytes};
mod approval_verifier;
pub use approval_verifier::ApprovalVerifier;
mod freshness_state;
mod receipt_approval_authority;
pub use receipt_approval_authority::ReceiptApprovalAuthority;
mod approval_permission;
pub use approval_permission::ApprovalPermission;

mod producer_registration;
pub use producer_registration::ProducerRegistration;

mod producer_registry;
pub use producer_registry::ProducerRegistry;

mod producer_attestation;
pub use producer_attestation::ProducerAttestation;

mod producer_signing_request;
pub use producer_signing_request::ProducerSigningRequest;

mod producer_signer;
pub use producer_signer::ProducerSigner;

mod producer_signing_profile;
pub use producer_signing_profile::{PRODUCER_SIGNING_DOMAIN, producer_signing_bytes};

mod verified_producer_evidence;
pub use verified_producer_evidence::VerifiedProducerEvidence;

mod producer_restore_context;
pub use producer_restore_context::ProducerRestoreContext;

mod producer_verifier;
pub use producer_verifier::ProducerVerifier;
