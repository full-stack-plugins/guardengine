use guard_protocol::{Digest, Identifier};
use guard_runner::CommandSpec;
use serde::Serialize;
use serde_json::json;
use std::collections::BTreeMap;

pub(crate) fn digest(value: &impl Serialize) -> Result<Digest, String> {
    guard_protocol::digest_json(&serde_json::to_vec(value).map_err(|e| e.to_string())?)
}
/// 宿主显式选择的原生档位；参数不是从候选报告或命令字符串解释。无Java对应类型。
#[derive(Clone, Debug)]
pub struct NativeVerifierCommand {
    pub verifier_id: Identifier,
    pub command: CommandSpec,
}
impl NativeVerifierCommand {
    /// 计算JCS({schemaVersion,args,directory,environment})摘要，必须等于编译config pin。
    /// directory使用Path的serde字符串；不接受有损路径转换。
    pub fn configuration_digest(command: &CommandSpec) -> Result<Digest, String> {
        let directory = command
            .directory
            .to_str()
            .ok_or("non-Unicode native directory")?;
        digest(
            &json!({"schemaVersion":"guard.native-command/v1","args":command.args,
            "directory":directory,"environment":command.environment}),
        )
    }
    /// 计算JCS显式环境映射的摘要；不表示外部DLL、JAR或OS状态已被捕获。
    pub fn environment_digest(environment: &BTreeMap<String, String>) -> Result<Digest, String> {
        digest(environment)
    }
}
