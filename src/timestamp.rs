//! Timestamp capture and rendering helpers.

use crate::cli::TimestampFormat;
use serde::Serialize;
use std::fmt;
use time::{OffsetDateTime, format_description::FormatItem, macros::format_description};

const RFC3339_MILLIS_UTC: &[FormatItem<'static>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z");

/// Canonical timestamp representation used by emitted records.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RecordTimestamp {
    #[serde(rename = "timestamp_unix_ms")]
    unix_ms: i64,
    #[serde(rename = "timestamp")]
    rfc3339_utc: String,
}

impl RecordTimestamp {
    /// Capture the current UTC timestamp.
    pub fn now() -> Self {
        Self::from_datetime(OffsetDateTime::now_utc())
    }

    /// Construct a timestamp from a Unix epoch expressed in milliseconds.
    pub fn from_unix_ms(unix_ms: i64) -> Self {
        let nanos = i128::from(unix_ms) * 1_000_000;
        let datetime = OffsetDateTime::from_unix_timestamp_nanos(nanos)
            .expect("unix_ms must fit into OffsetDateTime");
        Self::from_datetime(datetime)
    }

    fn from_datetime(datetime: OffsetDateTime) -> Self {
        let unix_ms = (datetime.unix_timestamp_nanos() / 1_000_000)
            .try_into()
            .expect("unix milliseconds must fit into i64");
        Self {
            unix_ms,
            rfc3339_utc: datetime
                .format(RFC3339_MILLIS_UTC)
                .expect("format RFC 3339 timestamp"),
        }
    }

    /// Unix epoch in milliseconds.
    pub fn unix_ms(&self) -> i64 {
        self.unix_ms
    }

    /// RFC 3339 / ISO 8601 representation in UTC with millisecond precision.
    pub fn rfc3339(&self) -> &str {
        &self.rfc3339_utc
    }

    /// Render according to a CLI-selected style.
    pub fn render(&self, format: TimestampFormat) -> RenderedTimestamp<'_> {
        match format {
            TimestampFormat::Unix => RenderedTimestamp::UnixMillis(self.unix_ms),
            TimestampFormat::Iso8601 => RenderedTimestamp::Iso8601(self.rfc3339()),
        }
    }
}

/// Borrowed rendering helper for human-oriented output.
pub enum RenderedTimestamp<'a> {
    UnixMillis(i64),
    Iso8601(&'a str),
}

impl fmt::Display for RenderedTimestamp<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnixMillis(unix_ms) => {
                let seconds = unix_ms.div_euclid(1_000);
                let millis = unix_ms.rem_euclid(1_000);
                write!(f, "{seconds}.{millis:03}")
            }
            Self::Iso8601(rfc3339) => f.write_str(rfc3339),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unix_ms_round_trips() {
        let ts = RecordTimestamp::from_unix_ms(1_746_072_812_345);
        assert_eq!(ts.unix_ms(), 1_746_072_812_345);
    }

    #[test]
    fn rfc3339_is_utc_with_millis() {
        let ts = RecordTimestamp::from_unix_ms(1_746_072_812_345);
        assert_eq!(ts.rfc3339(), "2025-05-01T04:13:32.345Z");
    }

    #[test]
    fn unix_render_uses_seconds_with_millis() {
        let ts = RecordTimestamp::from_unix_ms(1_746_072_812_345);
        assert_eq!(
            ts.render(TimestampFormat::Unix).to_string(),
            "1746072812.345"
        );
    }
}
