use guard_protocol::Digest;
use guard_runner::CommandSpec;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
/// 本次真实命令及批准模板的机械字面值；仅在私有来源原件中使用，不是任意shell。无Java对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceInvocation {
    pub executable: String,
    pub artifact_digest: Digest,
    pub argv: Vec<String>,
    pub working_directory: String,
    pub environment: BTreeMap<String, String>,
}
impl SourceInvocation {
    /// 只验证字面调用声明与批准profile一致，不构造派发凭据。
    pub fn verify_effective(&self, effective: &Self, profile: &str) -> Result<(), String> {
        let command = CommandSpec {
            executable: self.executable.clone().into(),
            executable_digest: self.artifact_digest.clone(),
            args: self.argv.clone(),
            directory: self.working_directory.clone().into(),
            environment: self.environment.clone(),
        };
        let expected = match profile {
            "guard.native-command/v1" => self.clone(),
            "guard.native-command/v2" => Self::capture(&crate::NativeVerifierCommandV2::render(
                &command,
                std::path::Path::new(&effective.working_directory),
            )?)?,
            _ => return Err("unknown source invocation profile".into()),
        };
        if &expected != effective {
            return Err("effective invocation differs from approved profile".into());
        }
        Ok(())
    }
    pub(crate) fn capture(command: &CommandSpec) -> Result<Self, String> {
        Ok(Self {
            executable: command
                .executable
                .to_str()
                .ok_or("non-Unicode executable")?
                .into(),
            artifact_digest: command.executable_digest.clone(),
            argv: command.args.clone(),
            working_directory: command
                .directory
                .to_str()
                .ok_or("non-Unicode directory")?
                .into(),
            environment: command.environment.clone(),
        })
    }
}
