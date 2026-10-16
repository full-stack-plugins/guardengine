use crate::native_verifier_command::digest;
use guard_protocol::{Digest, Identifier};
use guard_runner::CommandSpec;
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

/// 固定逻辑候选根、参数及环境模板的原生档位；无Java对应类型。
#[derive(Clone, Debug)]
pub struct NativeVerifierCommandV2 {
    pub verifier_id: Identifier,
    /// directory为逻辑原仓根；Runtime核验后才映射到私有候选根。
    pub command: CommandSpec,
}
fn validate_template(value: &str) -> Result<(), String> {
    let rest = value
        .replace("{{subject_root}}", "")
        .replace("{{subject_parent}}", "");
    if rest.contains("{{") || rest.contains("}}") || value.contains('\0') {
        return Err("unsupported native template token".into());
    }
    Ok(())
}
fn render_template(value: &str, root: &str, parent: &str) -> Result<String, String> {
    let mut result = String::new();
    let mut remaining = value;
    while let Some(index) = remaining.find("{{") {
        result.push_str(&remaining[..index]);
        remaining = &remaining[index..];
        if let Some(tail) = remaining.strip_prefix("{{subject_root}}") {
            result.push_str(root);
            remaining = tail;
        } else if let Some(tail) = remaining.strip_prefix("{{subject_parent}}") {
            result.push_str(parent);
            remaining = tail;
        } else {
            return Err("unsupported native template token".into());
        }
    }
    result.push_str(remaining);
    Ok(result)
}
impl NativeVerifierCommandV2 {
    /// 固定JCS逻辑workingDirectory与原始模板，不包含随机私有目录。
    pub fn configuration_digest(command: &CommandSpec) -> Result<Digest, String> {
        for argument in &command.args {
            validate_template(argument)?;
        }
        Self::environment_digest(&command.environment)?;
        digest(
            &json!({"schemaVersion":"guard.native-command/v2","workingDirectory":{"kind":"subject_root"},
            "argvTemplate":command.args,"environmentTemplate":command.environment}),
        )
    }
    /// 绑定原始环境模板；必须显式限定Git发现边界且暂不支持其他Git路由覆盖。
    pub fn environment_digest(environment: &BTreeMap<String, String>) -> Result<Digest, String> {
        if environment
            .get("GIT_CEILING_DIRECTORIES")
            .map(String::as_str)
            != Some("{{subject_parent}}")
        {
            return Err("v2 requires explicit GIT_CEILING_DIRECTORIES={{subject_parent}}".into());
        }
        let mut names = BTreeSet::new();
        for (key, value) in environment {
            let mut characters = key.bytes();
            if !characters
                .next()
                .is_some_and(|value| value.is_ascii_alphabetic() || value == b'_')
                || !characters.all(|value| value.is_ascii_alphanumeric() || value == b'_')
                || !names.insert(key.to_ascii_uppercase())
            {
                return Err("v2 requires ASCII environment names without case aliases".into());
            }
            if key.to_ascii_uppercase().starts_with("GIT_") && key != "GIT_CEILING_DIRECTORIES" {
                return Err("v2 does not support other GIT_* overrides".into());
            }
            validate_template(value)?;
        }
        digest(environment)
    }
    pub(crate) fn render(command: &CommandSpec, root: &Path) -> Result<CommandSpec, String> {
        Self::configuration_digest(command)?;
        let root = crate::native_root_path::native_root_path(root)?;
        let root_text = root.to_str().ok_or("non-Unicode materialized root")?;
        let parent_text = root
            .parent()
            .and_then(Path::to_str)
            .ok_or("private direct subject parent required")?;
        let mut rendered = command.clone();
        rendered.directory = root.to_path_buf();
        rendered.args = command
            .args
            .iter()
            .map(|value| render_template(value, root_text, parent_text))
            .collect::<Result<_, _>>()?;
        rendered.environment = command
            .environment
            .iter()
            .map(|(key, value)| Ok((key.clone(), render_template(value, root_text, parent_text)?)))
            .collect::<Result<_, String>>()?;
        Ok(rendered)
    }
}

#[cfg(test)]
mod tests {
    use super::NativeVerifierCommandV2;
    use guard_protocol::Digest;
    use guard_runner::CommandSpec;
    use std::{collections::BTreeMap, path::PathBuf};
    fn command() -> CommandSpec {
        CommandSpec {
            executable: PathBuf::from("verifier.exe"),
            executable_digest: Digest::new(format!("sha256:{}", "a".repeat(64))).unwrap(),
            args: vec!["--source={{subject_root}}/a.txt".into()],
            directory: PathBuf::from("logical-root"),
            environment: BTreeMap::from([(
                "GIT_CEILING_DIRECTORIES".into(),
                "{{subject_parent}}".into(),
            )]),
        }
    }
    #[test]
    fn closed_git_environment_rejects_missing_override_unicode_and_case_aliases() {
        let mut old = command();
        old.environment
            .insert("GIT_CEILING_DIRECTORIES".into(), "{{subject_root}}".into());
        assert!(NativeVerifierCommandV2::configuration_digest(&old).is_err());
        let mut input = command();
        input.environment.clear();
        assert!(NativeVerifierCommandV2::environment_digest(&input.environment).is_err());
        for key in [
            "GIT_DIR",
            "git_work_tree",
            "Git_Ceiling_Directories",
            "PÄTH",
            "9INVALID",
        ] {
            let mut input = command();
            input.environment.insert(key.into(), "value".into());
            assert!(
                NativeVerifierCommandV2::environment_digest(&input.environment).is_err(),
                "{key}"
            );
        }
        let mut input = command();
        input.environment.insert("PATH".into(), "a".into());
        input.environment.insert("Path".into(), "b".into());
        assert!(NativeVerifierCommandV2::environment_digest(&input.environment).is_err());
    }
    #[test]
    fn logical_pin_is_stable_but_literal_template_and_environment_are_bound() {
        let input = command();
        let original = NativeVerifierCommandV2::configuration_digest(&input).unwrap();
        let mut relocated = input.clone();
        relocated.directory = PathBuf::from("another-nonce");
        assert_eq!(
            original,
            NativeVerifierCommandV2::configuration_digest(&relocated).unwrap()
        );
        let root = std::env::current_dir().unwrap().join("私有根");
        let rendered = NativeVerifierCommandV2::render(&input, &root).unwrap();
        assert_eq!(
            rendered.args,
            [format!("--source={}/a.txt", rendered.directory.display())]
        );
        assert_eq!(
            rendered.environment["GIT_CEILING_DIRECTORIES"],
            rendered.directory.parent().unwrap().to_str().unwrap()
        );
        assert!(rendered.directory.ends_with("私有根"));
        assert_eq!(
            super::render_template(
                "--root={{subject_root}}",
                "/literal/{{subject_parent}}",
                "/parent"
            )
            .unwrap(),
            "--root=/literal/{{subject_parent}}"
        );
        relocated.args.push("--unapproved".into());
        assert_ne!(
            original,
            NativeVerifierCommandV2::configuration_digest(&relocated).unwrap()
        );
        let mut unsupported = input;
        unsupported.args.push("{{unknown_root}}".into());
        assert!(NativeVerifierCommandV2::configuration_digest(&unsupported).is_err());
    }
}
