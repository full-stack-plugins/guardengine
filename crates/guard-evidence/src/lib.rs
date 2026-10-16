//! 有界源码快照与显式信任锚签名；不提供执行、审批或操作系统隔离。
mod canonical;
mod execution;
mod file_input;
mod signing;
mod snapshot;
pub use canonical::digest_json;
pub use execution::verify_execution;
pub use file_input::{hash_file, read_json};
pub use signing::{keygen, sign_receipt, verify_receipt};
pub use snapshot::snapshot_project;
