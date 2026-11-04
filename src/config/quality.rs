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
        if !(1..=100).contains(&value) {
            return Err(crate::Error::general("Quality must be in [1, 100]"));
        }
        Ok(Quality(value))
    }
}

impl From<Quality> for u8 {
    fn from(value: Quality) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_try_from() {
        assert!(Quality::try_from(0).is_err());
        assert!(Quality::try_from(101).is_err());
        assert_eq!(Quality::try_from(50).unwrap().value(), 50);
        assert_eq!(Quality::try_from(100).unwrap().value(), 100);
    }

    #[test]
    fn test_quality_default() {
        let q = Quality::default();
        assert_eq!(q.value(), crate::DEFAULT_QUALITY);
    }

    #[test]
    fn test_quality_max() {
        let q = Quality::max();
        assert_eq!(q.value(), 100);
    }
}
