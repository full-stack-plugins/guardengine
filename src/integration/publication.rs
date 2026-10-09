//! Immutable per-attempt publication into a caller-protected private local directory.
//! Publication stores untrusted evidence; it does not make it current or eligible.
use super::*;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;
pub struct StagedAttempt {
    file: Option<NamedTempFile>,
    destination: PathBuf,
    root: PathBuf,
    root_identity: (u64, u64),
    content_digest: String,
}
pub struct PublishedAttempt {
    pub path: PathBuf,
    pub content_digest: String,
}
fn io_error(e: std::io::Error) -> IntegrationError {
    IntegrationError(format!("publication I/O: {}", e.kind()))
}
fn private_root(root: &Path) -> Result<(u64, u64), IntegrationError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = fs::symlink_metadata(root).map_err(io_error)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() || metadata.mode() & 0o077 != 0 {
            return fail("publication root must be a private non-symlink directory");
        }
        Ok((metadata.dev(), metadata.ino()))
    }
    #[cfg(not(unix))]
    {
        let _ = root;
        fail("publication platform not qualified")
    }
}
/// Stage bounded canonical bytes. Dropping this value never publishes a final attempt.
/// Caller must keep this storage root inaccessible to untrusted writers throughout its lifetime.
pub fn stage_attempt(
    root: &Path,
    envelope: &GuardRunEnvelope,
    profile: EvidenceProfile,
) -> Result<StagedAttempt, IntegrationError> {
    envelope.validate(profile)?;
    let identity = private_root(root)?;
    let root = fs::canonicalize(root).map_err(io_error)?;
    if private_root(&root)? != identity {
        return fail("publication root changed");
    }
    let bytes = serde_json::to_vec(envelope).map_err(|e| IntegrationError(e.to_string()))?;
    let destination = root.join(format!(
        "attempt-{:x}.json",
        Sha256::digest(envelope.run_id.as_bytes())
    ));
    let mut file = tempfile::Builder::new()
        .prefix(".guard-attempt-")
        .tempfile_in(&root)
        .map_err(io_error)?;
    file.write_all(&bytes).map_err(io_error)?;
    file.as_file().sync_all().map_err(io_error)?;
    Ok(StagedAttempt {
        file: Some(file),
        destination,
        root,
        root_identity: identity,
        content_digest: format!("sha256:{:x}", Sha256::digest(&bytes)),
    })
}
impl StagedAttempt {
    pub fn destination(&self) -> &Path {
        &self.destination
    }
    /// Never overwrites any existing final path, including a symlink or another attempt.
    /// A final path may exist if directory synchronization fails: reconcile its digest before retrying.
    pub fn publish(mut self) -> Result<PublishedAttempt, IntegrationError> {
        if private_root(&self.root)? != self.root_identity {
            return fail("publication root changed before commit");
        }
        let _file = self
            .file
            .take()
            .ok_or_else(|| IntegrationError("publication already consumed".into()))?
            .persist_noclobber(&self.destination)
            .map_err(|e| io_error(e.error))?;
        fs::File::open(&self.root).and_then(|dir|dir.sync_all()).map_err(|_|IntegrationError("attempt linked but directory synchronization failed; reconcile final bytes before retry".into()))?;
        Ok(PublishedAttempt {
            path: self.destination.clone(),
            content_digest: self.content_digest.clone(),
        })
    }
}

impl Drop for StagedAttempt {
    fn drop(&mut self) {
        // Never let pathname-based tempfile cleanup remove an entry in a replacement directory.
        // The orphan in the old store is safer than unlinking an unrelated replacement entry.
        if private_root(&self.root).ok() != Some(self.root_identity) {
            if let Some(file) = self.file.take() {
                let _ = file.keep();
            }
        }
    }
}
