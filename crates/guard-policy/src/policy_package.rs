use crate::{PolicyManifest, capabilities, paths};
use guard_protocol::Digest;
use serde::Serialize;
use sha2::{Digest as _, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
/// 安全加载后固定的策略与数据原件；没有Deserialize，不从结果推导批准。无Java对应类型。
#[derive(Clone, Debug)]
pub struct PolicyPackage {
    manifest: PolicyManifest,
    files: BTreeMap<String, Vec<u8>>,
    capabilities: Vec<u8>,
    digest: Digest,
}
pub(crate) fn hash(bytes: &[u8]) -> Digest {
    Digest::new(format!("sha256:{:x}", Sha256::digest(bytes))).expect("SHA256 format")
}
pub(crate) fn json_hash(value: &impl Serialize) -> Result<Digest, String> {
    guard_protocol::digest_json(&serde_json::to_vec(value).map_err(|e| e.to_string())?)
}
impl PolicyPackage {
    /// 从显式目录和闭合manifest加载全部固定原件；源包后续修改不替换本对象中的批准候选。
    pub fn load(root: &Path, manifest_bytes: &[u8]) -> Result<Self, String> {
        if manifest_bytes.len() > 1024 * 1024 {
            return Err("policy manifest exceeds 1MiB".into());
        }
        let root = paths::safe(root)?;
        if !root.is_dir() {
            return Err("policy package directory required".into());
        }
        let raw = guard_protocol::canonicalize(manifest_bytes)?;
        let manifest: PolicyManifest = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
        if manifest.schema_version != 1
            || manifest.files.is_empty()
            || manifest.files.len() > 64
            || !valid_query(&manifest.entrypoint)
        {
            return Err("unsupported policy manifest/query".into());
        }
        let mut files = BTreeMap::new();
        let mut portable = BTreeSet::new();
        let mut prior = None;
        let mut total = 0usize;
        for file in &manifest.files {
            paths::relative(&file.path)?;
            if prior.as_ref().is_some_and(|p: &String| p >= &file.path)
                || !portable.insert(file.path.to_ascii_lowercase())
                || !(file.path.ends_with(".rego")
                    || file.path.split('/').next_back() == Some("data.json"))
            {
                return Err("sorted unique Rego/data.json policy file set required".into());
            }
            prior = Some(file.path.clone());
            let bytes = paths::read(&root.join(&file.path), 1024 * 1024)?;
            total += bytes.len();
            if total > 4 * 1024 * 1024 || hash(&bytes) != file.digest {
                return Err("policy package hash/size mismatch".into());
            }
            if file.path.ends_with(".json") {
                guard_protocol::canonicalize(&bytes)?;
            } else {
                std::str::from_utf8(&bytes).map_err(|e| e.to_string())?;
            }
            files.insert(file.path.clone(), bytes);
        }
        paths::relative(&manifest.capabilities.path)?;
        if portable.contains(&manifest.capabilities.path.to_ascii_lowercase()) {
            return Err("capabilities must be separate from policy bundle files".into());
        }
        let caps = paths::read(&root.join(&manifest.capabilities.path), 1024 * 1024)?;
        if hash(&caps) != manifest.capabilities.digest {
            return Err("capabilities digest mismatch".into());
        }
        capabilities::validate(&caps)?;
        let digest = json_hash(&manifest)?;
        Ok(Self {
            manifest,
            files,
            capabilities: caps,
            digest,
        })
    }
    /// 返回唯一manifest身份摘要：JCS元数据+每个原始文件摘要+能力摘要。
    pub fn digest(&self) -> &Digest {
        &self.digest
    }
    /// 返回只读manifest；本地加载本身不是批准。
    pub fn manifest(&self) -> &PolicyManifest {
        &self.manifest
    }
    pub(crate) fn files(&self) -> &BTreeMap<String, Vec<u8>> {
        &self.files
    }
    pub(crate) fn capabilities(&self) -> &[u8] {
        &self.capabilities
    }
}
pub(crate) fn valid_query(query: &str) -> bool {
    query.strip_prefix("data.").is_some_and(|tail| {
        !tail.is_empty()
            && tail.len() <= 256
            && tail.split('.').all(|segment| {
                !segment.is_empty()
                    && segment.bytes().enumerate().all(|(i, b)| {
                        b == b'_' || b.is_ascii_alphabetic() || (i > 0 && b.is_ascii_digit())
                    })
            })
    })
}
