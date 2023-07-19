use std::fmt::{Display, Write};

use time::{OffsetDateTime, PrimitiveDateTime, UtcOffset};

use super::Result;

/// Represents a possible embedded time format flag.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum EmbeddedTimeFlag {
    /// A relative embedded time.
    #[default]
    Relative = b'R',
    /// An embedded time containing the time in a short format.
    TimeShort = b't',
    /// An embedded time containing the time in a long format.
    TimeLong = b'T',
    /// An embedded time containing the date in a short format.
    DateShort = b'd',
    /// An embedded time containing the date in a long format.
    DateLong = b'D',
    /// An embedded time containing the date and time in a short format.
    DateTimeShort = b'f',
    /// An embedded time containing the date and time in a long format.
    DateTimeLong = b'F',
}

impl Display for EmbeddedTimeFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char(char::from(*self as u8))
    }
}

/// Represents an embedded Discord timestamp string.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EmbeddedTime {
    /// The internal time value.
    time: OffsetDateTime,
    /// The embedded time's format flag.
    flag: Option<EmbeddedTimeFlag>,
}

impl EmbeddedTime {
    /// Creates a new embedded time with the given flag.
    pub fn new_with(unix: i64, flag: EmbeddedTimeFlag) -> Result<Self> {
        let time = OffsetDateTime::from_unix_timestamp(unix)?;

        Ok(Self { time, flag: Some(flag) })
    }

    /// Creates a new embedded time.
    pub fn new(unix: i64) -> Result<Self> {
        let time = OffsetDateTime::from_unix_timestamp(unix)?;

        Ok(Self { time, flag: None })
    }
}

impl From<PrimitiveDateTime> for EmbeddedTime {
    fn from(value: PrimitiveDateTime) -> Self { value.assume_offset(UtcOffset::UTC).into() }
}

impl From<OffsetDateTime> for EmbeddedTime {
    fn from(time: OffsetDateTime) -> Self { Self { time, flag: None } }
}

impl Display for EmbeddedTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let unix = self.time.unix_timestamp();
        let flag = self.flag.unwrap_or_default();

        write!(f, "<t:{unix}:{flag}>")
    }
}
