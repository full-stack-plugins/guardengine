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
