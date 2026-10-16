use crate::git_client::GitClient;
use guard_protocol::GitOid;
use sha2::{Digest, Sha256};
/// 从 Git 读取原始对象体并独立核验 header+bytes 的 SHA1/SHA256，不能信任对象文件名。
pub(crate) fn read(
    client: &GitClient,
    kind: &str,
    oid: &GitOid,
    limit: usize,
) -> Result<Vec<u8>, String> {
    let bytes = client.run(&["cat-file", kind, oid.as_str()], limit, true)?;
    let header = format!("{kind} {}\0", bytes.len());
    let actual = if oid.as_str().len() == 40 {
        let mut hasher = sha1::Sha1::new();
        hasher.update(header.as_bytes());
        hasher.update(&bytes);
        format!("{:x}", hasher.finalize())
    } else {
        let mut hasher = Sha256::new();
        hasher.update(header.as_bytes());
        hasher.update(&bytes);
        format!("{:x}", hasher.finalize())
    };
    if actual != oid.as_str() {
        return Err(format!("Git {kind} object hash mismatch"));
    }
    Ok(bytes)
}
