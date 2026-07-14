use std::num::NonZeroU16;

use serde::{Deserialize, Serialize};

use crate::DEFAULT_QUALITY;

/// Configuration for the JPEG codec.
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct JpegCodecConfiguration {
    pub quality: Quality,
    pub color_config: ColorConfig,
}

impl Serialize for JpegCodecConfiguration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let raw = JpegCodecConfigurationRaw::from(*self);
        raw.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for JpegCodecConfiguration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = JpegCodecConfigurationRaw::deserialize(deserializer)?;
        JpegCodecConfiguration::try_from(raw).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ColorSpace {
    #[default]
    YCbCr,
    Rgb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromaSubsampling {
    /// No subsampling, 1 chrominance sample per pixel.
    Cs4_4_4,
    /// 2x1 (horizontal) chrominance subsampling; 1 chrominance sample for every 2x1 block of pixels.
    Cs4_2_2,
    /// 2x2 (square) chrominance subsampling; 1 chrominance sample for every 2x2 block of pixels.
    Cs4_2_0,
    /// 1x2 (vertical) chrominance subsampling; 1 chrominance sample for every 1x2 block of pixels.
    /// May incur a performance cost.
    Cs4_4_0,
    // Cs4_4_1,
    // Cs4_1_1,
}

type ChomaSubSamplingArray = [[u8; 2]; 3];

impl ChromaSubsampling {
    pub fn none() -> Self {
        ChromaSubsampling::Cs4_4_4
    }

    pub fn default_ycbcr() -> Self {
        ChromaSubsampling::Cs4_2_0
    }

    fn to_array(self) -> ChomaSubSamplingArray {
        match self {
            ChromaSubsampling::Cs4_4_4 => [[1, 1], [1, 1], [1, 1]],
            ChromaSubsampling::Cs4_2_2 => [[2, 1], [1, 1], [1, 1]],
            ChromaSubsampling::Cs4_2_0 => [[2, 2], [1, 1], [1, 1]],
            ChromaSubsampling::Cs4_4_0 => [[1, 2], [1, 1], [1, 1]],
            // ChromaSubsampling::Cs4_4_1 => [[1, 4], [1, 1], [1, 1]],
            // ChromaSubsampling::Cs4_1_1 => [[4, 4], [1, 1], [1, 1]],
        }
    }

    /// None if not valid.
    fn from_array(arr: &ChomaSubSamplingArray) -> Option<Self> {
        match arr {
            [[1, 1], [1, 1], [1, 1]] => Some(ChromaSubsampling::Cs4_4_4),
            [[2, 1], [1, 1], [1, 1]] => Some(ChromaSubsampling::Cs4_2_2),
            [[2, 2], [1, 1], [1, 1]] => Some(ChromaSubsampling::Cs4_2_0),
            [[1, 2], [1, 1], [1, 1]] => Some(ChromaSubsampling::Cs4_4_0),
            // [[1, 4], [1, 1], [1, 1]] => Some(ChromaSubsampling::Cs4_4_1),
            // [[4, 4], [1, 1], [1, 1]] => Some(ChromaSubsampling::Cs4_1_1),
            _ => None,
        }
    }
}

impl Serialize for ChromaSubsampling {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let arr = self.to_array();
        arr.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ChromaSubsampling {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let arr = ChomaSubSamplingArray::deserialize(deserializer)?;
        Self::from_array(&arr).ok_or_else(|| {
            serde::de::Error::custom(format!("invalid chroma subsampling array: {:?}", arr))
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct JpegShape {
    pub width: NonZeroU16,
    pub height: NonZeroU16,
    pub is_rgb: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorConfig {
    YCbCr { subsampling: ChromaSubsampling },
    Rgb,
    Grayscale,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct ColorConfigRaw {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    encoded_color_space: Option<ColorSpace>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    subsampling: Option<ChromaSubsampling>,
}

impl TryFrom<ColorConfigRaw> for ColorConfig {
    type Error = &'static str;

    fn try_from(value: ColorConfigRaw) -> Result<Self, Self::Error> {
        match value.encoded_color_space {
            Some(ColorSpace::YCbCr) => {
                let subsampling = value.subsampling.unwrap_or(ChromaSubsampling::Cs4_2_0);
                Ok(ColorConfig::YCbCr { subsampling })
            }
            Some(ColorSpace::Rgb) => {
                if let Some(subsampling) = value.subsampling
                    && subsampling != ChromaSubsampling::Cs4_4_4
                {
                    return Err("Invalid subsampling for RGB");
                }
                Ok(ColorConfig::Rgb)
            }
            None => {
                if let Some(subsampling) = value.subsampling
                    && subsampling != ChromaSubsampling::Cs4_4_4
                {
                    return Err("Invalid subsampling for grayscale");
                }
                Ok(ColorConfig::Grayscale)
            }
        }
    }
}

impl From<ColorConfig> for ColorConfigRaw {
    fn from(value: ColorConfig) -> Self {
        match value {
            ColorConfig::YCbCr { subsampling: ss } => {
                // compact representation where possible
                let subsampling = if ss == ChromaSubsampling::default_ycbcr() {
                    None
                } else {
                    Some(ss)
                };
                ColorConfigRaw {
                    encoded_color_space: Some(ColorSpace::YCbCr),
                    subsampling,
                }
            }
            ColorConfig::Rgb => ColorConfigRaw {
                encoded_color_space: Some(ColorSpace::Rgb),
                subsampling: None,
            },
            ColorConfig::Grayscale => ColorConfigRaw {
                encoded_color_space: None,
                subsampling: None,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct JpegCodecConfigurationRaw {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    quality: Option<u8>,
    #[serde(flatten)]
    color_config: ColorConfigRaw,
}

impl TryFrom<JpegCodecConfigurationRaw> for JpegCodecConfiguration {
    type Error = &'static str;

    fn try_from(value: JpegCodecConfigurationRaw) -> Result<Self, Self::Error> {
        let color_config = ColorConfig::try_from(value.color_config)?;
        Ok(JpegCodecConfiguration {
            quality: Quality::try_new(value.quality)?,
            color_config,
        })
    }
}

impl From<JpegCodecConfiguration> for JpegCodecConfigurationRaw {
    fn from(value: JpegCodecConfiguration) -> Self {
        // compact representation where possible
        let quality = if value.quality == Quality::default() {
            None
        } else {
            Some(value.quality.into())
        };
        JpegCodecConfigurationRaw {
            quality,
            color_config: value.color_config.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub struct Quality(u8);

impl Default for Quality {
    fn default() -> Self {
        Quality(DEFAULT_QUALITY)
    }
}

impl Quality {
    pub fn max() -> Self {
        Quality(100)
    }

    pub fn try_new(quality: Option<u8>) -> Result<Self, &'static str> {
        if let Some(q) = quality {
            Self::try_from(q)
        } else {
            Ok(Quality(DEFAULT_QUALITY))
        }
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for Quality {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value > 100 {
            return Err("Quality must be between 1 and 100");
        }
        Ok(Quality(value))
    }
}

impl From<Quality> for u8 {
    fn from(value: Quality) -> Self {
        value.0
    }
}
