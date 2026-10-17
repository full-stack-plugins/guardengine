//! 已注册计划的本地执行闭环；没有领域质量裁决、可信授权或OS沙箱。
mod artifact_ref;
mod check_failure;
mod dispatch_outcome;
mod execution_observation;
mod invocation_receipt;
mod materialization_receipt;
mod native_root_path;
mod native_verifier_command;
mod native_verifier_command_v2;
mod runtime;
pub use artifact_ref::ArtifactRef;
pub use dispatch_outcome::DispatchOutcome;
pub use execution_observation::ExecutionObservation;
pub use invocation_receipt::InvocationReceipt;
pub use materialization_receipt::MaterializationReceipt;
pub use native_verifier_command::NativeVerifierCommand;
pub use native_verifier_command_v2::NativeVerifierCommandV2;
pub use runtime::Runtime;

mod dispatch_completion;
pub use dispatch_completion::DispatchCompletion;
mod dispatch_with_receipt;
pub use dispatch_with_receipt::DispatchWithReceipt;

mod source_invocation;
pub use source_invocation::SourceInvocation;
mod dispatch_provenance;
pub use dispatch_provenance::DispatchProvenance;

mod dispatch_profile;
