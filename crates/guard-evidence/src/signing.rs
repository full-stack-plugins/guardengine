use crate::{
    canonical::canonical,
    file_input::{read, safe_path},
};
use ring::{
    rand::SystemRandom,
    signature::{self, Ed25519KeyPair, KeyPair},
};
use serde_json::{Value, json};
use std::{fs, io::Write, path::Path};

const SCHEMA: &str = "guard.signed-evidence/v1";
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn unhex(text: &str, size: usize) -> Result<Vec<u8>, String> {
    if text.len() != size * 2
        || !text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("invalid key or signature encoding".into());
    }
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).map_err(|e| e.to_string()))
        .collect()
}
fn message(payload: &Value) -> Result<Vec<u8>, String> {
    if !payload.is_object() {
        return Err("receipt payload must be an object".into());
    }
    let mut bytes = format!("{SCHEMA}\n").into_bytes();
    bytes.extend(canonical(payload)?);
    Ok(bytes)
}
fn new_file(path: &Path, bytes: &[u8], private: bool) -> Result<(), String> {
    if !path.is_absolute() {
        return Err("absolute key path required".into());
    }
    safe_path(path.parent().ok_or("key parent required")?)?;
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(if private { 0o600 } else { 0o644 });
    }
    #[cfg(not(unix))]
    let _ = private;
    let mut file = options.open(path).map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|e| e.to_string())
}
/// 显式创建全新 Ed25519 私钥和公钥文件；不覆盖已有内容，不产生审批。
pub fn keygen(private: &Path, public: &Path) -> Result<(), String> {
    if fs::symlink_metadata(private).is_ok() || fs::symlink_metadata(public).is_ok() {
        return Err("key outputs already exist".into());
    }
    let pkcs8 = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new())
        .map_err(|_| "key generation failed")?;
    let pair = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).map_err(|_| "key generation failed")?;
    new_file(private, pkcs8.as_ref(), true)?;
    if let Err(error) = new_file(
        public,
        format!("{}\n", hex(pair.public_key().as_ref())).as_bytes(),
        false,
    ) {
        let _ = fs::remove_file(private);
        return Err(error);
    }
    Ok(())
}
/// 使用调用者显式选择的 PKCS#8 密钥签署 payload；不校验业务声明或运行沙箱。
pub fn sign_receipt(payload: &Value, private_key: &Path) -> Result<Value, String> {
    let pair = Ed25519KeyPair::from_pkcs8(&read(private_key, 4096)?)
        .map_err(|_| "invalid Ed25519 private key")?;
    let signature = pair.sign(&message(payload)?);
    Ok(
        json!({"schema_version":SCHEMA,"payload":payload,"signature":{"algorithm":"Ed25519","public_key":hex(pair.public_key().as_ref()),"value":hex(signature.as_ref())}}),
    )
}
/// 以显式公钥文件核验签名，成功返回 payload；不从回执自选信任锚。
pub fn verify_receipt(envelope: &Value, public_key: &Path) -> Result<Value, String> {
    if envelope.as_object().is_none_or(|o| o.len() != 3)
        || envelope["schema_version"] != SCHEMA
        || envelope["signature"]
            .as_object()
            .is_none_or(|o| o.len() != 3)
        || envelope["signature"]["algorithm"] != "Ed25519"
    {
        return Err("unsupported signed evidence envelope".into());
    }
    let bytes = read(public_key, 256)?;
    let pinned = unhex(
        std::str::from_utf8(&bytes)
            .map_err(|_| "invalid public key text")?
            .trim(),
        32,
    )?;
    let declared = unhex(
        envelope["signature"]["public_key"]
            .as_str()
            .ok_or("public key missing")?,
        32,
    )?;
    if pinned != declared {
        return Err("evidence key differs from explicit trust anchor".into());
    }
    let signature = unhex(
        envelope["signature"]["value"]
            .as_str()
            .ok_or("signature missing")?,
        64,
    )?;
    signature::UnparsedPublicKey::new(&signature::ED25519, &pinned)
        .verify(&message(&envelope["payload"])?, &signature)
        .map_err(|_| "evidence signature verification failed")?;
    Ok(envelope["payload"].clone())
}
