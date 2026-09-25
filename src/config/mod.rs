use serde::{Deserialize, Serialize};
mod quality;
pub use quality::Quality;
mod raw;
use raw::{ColorConfigRaw, ConfigRaw};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorSpace {
    Grayscale,
    Rgb,
    YCbCr,
}

impl ColorSpace {
    pub fn components(&self) -> usize {
        match self {
            ColorSpace::Grayscale => 1,
            ColorSpace::Rgb => 3,
            ColorSpace::YCbCr => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(try_from = "ConfigRaw", into = "ConfigRaw")]
pub struct JpegCodecConfig {
    pub(crate) quality: Quality,
    pub(crate) color_config: ColorConfig,
}

impl JpegCodecConfig {
    pub fn new(quality: Quality, color_config: ColorConfig) -> Self {
        Self {
            quality,
            color_config,
        }
    }

    /// Quality, guaranteed to be in `[0, 100]`.
    pub fn quality(&self) -> u8 {
        *self.quality
    }

    pub fn color_config(&self) -> ColorConfig {
        self.color_config
    }

    /// Get the ratio of luminance pixels to chrominance pixels, if any subsampling is used.
    pub fn subsampling(&self) -> Option<SamplingRatios> {
        match self.color_config {
            ColorConfig::YCbCr { subsampling } | ColorConfig::RgbToYCbCr { subsampling } => {
                if subsampling.horizontal.value() == 1 && subsampling.vertical.value() == 1 {
                    None
                } else {
                    Some(subsampling)
                }
            }
            _ => None,
        }
    }

    /// Get the width and height in pixels of the minimum coding unit (MCU) for this configuration.
    pub fn mcu_shape(&self) -> (u16, u16) {
        self.color_config.mcu_shape()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct SamplingRatio(u8);

impl SamplingRatio {
    pub fn value(&self) -> u8 {
        self.0
    }
}

impl Default for SamplingRatio {
    fn default() -> Self {
        SamplingRatio(1)
    }
}

impl From<SamplingRatio> for u8 {
    fn from(ratio: SamplingRatio) -> Self {
        ratio.0
    }
}

impl TryFrom<u8> for SamplingRatio {
    type Error = crate::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if (1..=4).contains(&value) {
            Ok(SamplingRatio(value))
        } else {
            Err(Self::Error::general("Subsampling ratio must be in [1, 4]"))
        }
    }
}

/// Pixel subsampling ratios.
#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(
    from = "(SamplingRatio, SamplingRatio)",
    into = "(SamplingRatio, SamplingRatio)"
)]
pub struct SamplingRatios {
    pub horizontal: SamplingRatio,
    pub vertical: SamplingRatio,
}

impl SamplingRatios {
    pub fn new(horizontal: SamplingRatio, vertical: SamplingRatio) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }

    pub fn try_new(horizontal: u8, vertical: u8) -> Result<Self, crate::Error> {
        Ok(Self {
            horizontal: SamplingRatio::try_from(horizontal)?,
            vertical: SamplingRatio::try_from(vertical)?,
        })
    }

    pub fn values(&self) -> (u8, u8) {
        (self.horizontal.0, self.vertical.0)
    }
}

impl From<SamplingRatios> for (SamplingRatio, SamplingRatio) {
    fn from(ratios: SamplingRatios) -> Self {
        (ratios.horizontal, ratios.vertical)
    }
}

impl From<(SamplingRatio, SamplingRatio)> for SamplingRatios {
    fn from(ratios: (SamplingRatio, SamplingRatio)) -> Self {
        Self {
            horizontal: ratios.0,
            vertical: ratios.1,
        }
    }
}

/// Configuration for color transforms and chroma subsampling.
///
/// Encoded and decoded data may be grayscale, RGB, or YCbCr.
/// Only YCbCr-encoded data may be subsampled.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(try_from = "ColorConfigRaw", into = "ColorConfigRaw")]
pub enum ColorConfig {
    Grayscale,
    Rgb,
    YCbCr {
        /// Luminance pixels per chrominance pixel.
        subsampling: SamplingRatios,
    },
    RgbToYCbCr {
        /// Luminance pixels per chrominance pixel.
        subsampling: SamplingRatios,
    },
}

impl ColorConfig {
    /// Width and height of the MCU, in pixels.
    pub fn mcu_shape(&self) -> (u16, u16) {
        match self {
            ColorConfig::Grayscale => (8, 8),
            ColorConfig::Rgb => (8, 8),
            ColorConfig::YCbCr { subsampling } | ColorConfig::RgbToYCbCr { subsampling } => {
                let (h, v) = subsampling.values();
                (h as u16 * 8, v as u16 * 8)
            }
        }
    }

    /// Decoded and encoded color spaces.
    pub fn color_spaces(&self) -> (ColorSpace, ColorSpace) {
        match self {
            ColorConfig::Grayscale => (ColorSpace::Grayscale, ColorSpace::Grayscale),
            ColorConfig::Rgb => (ColorSpace::Rgb, ColorSpace::Rgb),
            ColorConfig::YCbCr { .. } => (ColorSpace::YCbCr, ColorSpace::YCbCr),
            ColorConfig::RgbToYCbCr { .. } => (ColorSpace::Rgb, ColorSpace::YCbCr),
        }
    }
}
