use std::fmt::Write;

use crate::prelude::*;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimestampFlag {
    TimeShort,
    TimeLong,
    DateShort,
    DateLong,
    DateTimeShort,
    DateTimeLong,
    #[default]
    Relative,
}

impl Display for TimestampFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char(match self {
            Self::TimeShort => 't',
            Self::TimeLong => 'T',
            Self::DateShort => 'd',
            Self::DateLong => 'D',
            Self::DateTimeShort => 'f',
            Self::DateTimeLong => 'F',
            Self::Relative => 'R',
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timestamp(i64, Option<TimestampFlag>);

impl Timestamp {
    pub const fn new(ms: i64) -> Self {
        Self(ms, None)
    }
    pub fn new_in(ms: i64) -> Self {
        let now = Utc::now().timestamp_millis();

        Self::new(now.saturating_add(ms))
    }

    pub const fn with_flag(mut self, flag: TimestampFlag) -> Self {
        self.1 = Some(flag);
        self
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self::new_in(0)
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(value: DateTime<Utc>) -> Self {
        Self::new(value.timestamp_millis())
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = self.0 / 1000;
        let k = self.1.unwrap_or_default();

        write!(f, "<t:{n}:{k}>")
    }
}
