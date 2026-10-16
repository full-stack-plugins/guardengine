use serde::{Deserialize, Deserializer, Serialize};
/// 显式 UTC 时间；校验日历、范围和纳秒精度，不证明时钟可信。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct UtcTimestamp(String);
impl UtcTimestamp {
    /// 校验并创建值；参数为原始文本，返回合法值或具体错误。
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        if crate::validation::valid_utc(&value) {
            Ok(Self(value))
        } else {
            Err("invalid UtcTimestamp".into())
        }
    }
    /// 比较 UTC 时刻；原文相等性独立保留，`.1Z` 与 `.10Z` 的时刻相同但文本不同。
    pub fn compare_instant(&self, other: &Self) -> std::cmp::Ordering {
        crate::validation::compare_time(self, other)
    }
    /// 返回原始文本，不做大小写折叠或 Unicode 归一化。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl<'de> Deserialize<'de> for UtcTimestamp {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}
