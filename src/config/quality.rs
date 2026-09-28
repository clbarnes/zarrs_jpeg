use std::ops::Deref;

use serde::{Deserialize, Serialize};

/// Instantiate with TryFrom<u8>. Must be <100.
#[derive(Debug, Clone, Copy, Serialize, Eq, PartialEq, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct Quality(u8);

impl Deref for Quality {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Default for Quality {
    fn default() -> Self {
        Quality(crate::DEFAULT_QUALITY)
    }
}

impl Quality {
    /// Maximum quality (still lossy).
    pub fn max() -> Self {
        Quality(100)
    }

    /// Create from a u8, using the default quality if None is given.
    ///
    /// To skip the None check, use `Quality::try_from` directly.
    pub fn try_new(quality: Option<u8>) -> Result<Self, crate::Error> {
        if let Some(q) = quality {
            Self::try_from(q)
        } else {
            Ok(Quality(crate::DEFAULT_QUALITY))
        }
    }

    /// Get the quality value as a u8; guaranteed to be <=100.
    pub fn value(&self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for Quality {
    type Error = crate::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value > 100 {
            return Err(crate::Error::general("Quality must be between 0 and 100"));
        }
        Ok(Quality(value))
    }
}

impl From<Quality> for u8 {
    fn from(value: Quality) -> Self {
        value.0
    }
}
