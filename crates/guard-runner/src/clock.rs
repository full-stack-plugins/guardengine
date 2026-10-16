use chrono::{DateTime, SecondsFormat, Utc};
use guard_protocol::UtcTimestamp;
use std::time::{SystemTime, UNIX_EPOCH};
/// 返回进程观察时的本机UTC墙钟，不用于运行排序或可信授权时间。
pub(crate) fn now() -> Result<UtcTimestamp, String> {
    let millis = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis(),
    )
    .map_err(|e| e.to_string())?;
    from_millis(millis)
}
fn from_millis(millis: i64) -> Result<UtcTimestamp, String> {
    let date = DateTime::<Utc>::from_timestamp_millis(millis)
        .ok_or("UTC observation outside calendar range")?;
    UtcTimestamp::new(date.to_rfc3339_opts(SecondsFormat::Millis, true))
}

#[cfg(test)]
mod tests {
    #[test]
    fn utc_observation_is_exact_and_out_of_protocol_calendar_is_rejected() {
        assert_eq!(
            super::from_millis(0).unwrap().as_str(),
            "1970-01-01T00:00:00.000Z"
        );
        assert_eq!(
            super::from_millis(951782400123).unwrap().as_str(),
            "2000-02-29T00:00:00.123Z"
        );
        assert!(super::from_millis(i64::MAX).is_err());
        assert!(super::from_millis(253402300800000).is_err());
    }
}
