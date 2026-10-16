use serde::{Deserialize, Deserializer, Serialize};
/// 无歧义十进制运行序列；按整数比较而非客户端时间排序。 无 Java 对应类型。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct RunSequence(String);
impl RunSequence {
    /// 校验并创建值；参数为原始文本，返回合法值或具体错误。
    pub fn new(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        if !value.is_empty()
            && value.len() <= 20
            && value.bytes().all(|b| b.is_ascii_digit())
            && (value == "0" || !value.starts_with('0'))
            && value.parse::<u64>().is_ok()
        {
            Ok(Self(value))
        } else {
            Err("invalid RunSequence".into())
        }
    }
    /// 返回原始文本，不做大小写折叠或 Unicode 归一化。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl<'de> Deserialize<'de> for RunSequence {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl Ord for RunSequence {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // 字符串已验证为无前导零的 u64 十进制；长度优先等价于整数顺序。
        self.0
            .len()
            .cmp(&other.0.len())
            .then_with(|| self.0.cmp(&other.0))
    }
}
impl PartialOrd for RunSequence {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
