use crate::{PublicSummary, RecordState};
use guard_protocol::Digest;
/// 私有数据库记录，不接受数据库内容直接拼接任意路径。无 Java 对应类型。
pub(crate) struct Record {
    pub operation: String,
    pub digest: String,
    pub size: u64,
    pub retain_until: u64,
    pub state: RecordState,
}
impl Record {
    pub(crate) fn summary(&self) -> Result<PublicSummary, String> {
        Ok(PublicSummary {
            operation_digest: Digest::new(self.operation.clone())?,
            content_digest: Digest::new(self.digest.clone())?,
            size: self.size,
            state: self.state,
        })
    }
    pub(crate) fn filename(&self) -> Result<String, String> {
        self.summary()?;
        Ok(format!(
            "{}-{}.blob",
            &self.operation[7..],
            &self.digest[7..]
        ))
    }
}
