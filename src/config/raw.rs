use super::ColorSpace;
use super::{ColorConfig, JpegCodecConfig, Quality, SamplingRatio, SamplingRatios};
use serde::{Deserialize, Serialize};

/// Representation of the stored JSON object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ConfigRaw {
    quality: Quality,
    #[serde(flatten)]
    color_config: ColorConfigRaw,
}

impl TryFrom<ConfigRaw> for JpegCodecConfig {
    type Error = crate::Error;

    fn try_from(value: ConfigRaw) -> Result<Self, Self::Error> {
        let color_config = ColorConfig::try_from(value.color_config)?;
        Ok(JpegCodecConfig {
            quality: value.quality,
            color_config,
        })
    }
}

impl From<JpegCodecConfig> for ConfigRaw {
    fn from(value: JpegCodecConfig) -> Self {
        let color_config = value.color_config.into();
        ConfigRaw {
            quality: value.quality,
            color_config,
        }
    }
}

/// Representation of the stored JSON fields for color configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ColorConfigRaw {
    #[serde(skip_serializing_if = "Option::is_none")]
    encoded_color_space: Option<ColorSpace>,
    #[serde(skip_serializing_if = "Option::is_none")]
    decoded_color_space: Option<ColorSpace>,
    subsampling: Vec<SamplingRatios>,
}

impl TryFrom<ColorConfigRaw> for ColorConfig {
    type Error = crate::Error;

    fn try_from(value: ColorConfigRaw) -> Result<Self, Self::Error> {
        let enc_cspace = value.encoded_color_space.unwrap_or(ColorSpace::Grayscale);
        let dec_cspace = value.decoded_color_space.unwrap_or_else(|| {
            if enc_cspace == ColorSpace::Grayscale {
                ColorSpace::Grayscale
            } else {
                ColorSpace::Rgb
            }
        });
        let enc_components = enc_cspace.components();
        if enc_components != dec_cspace.components() {
            return Err(Self::Error::general(
                "Encoded and decoded color spaces must have the same number of components",
            ));
        }

        if enc_components != value.subsampling.len() {
            return Err(Self::Error::general(
                "Subsampling must have the same number of components as the encoded color space",
            ));
        }

        const TRIVIAL_SUBSAMPLING: SamplingRatios = SamplingRatios {
            horizontal: SamplingRatio(1),
            vertical: SamplingRatio(1),
        };

        let mut subs = value.subsampling;
        while subs.len() > 1 {
            let sub = subs.pop().expect("already checked length");
            if sub != TRIVIAL_SUBSAMPLING {
                return Err(Self::Error::general(
                    "Only the first component can have non-trivial subsampling",
                ));
            }
        }
        let subsampling = subs.pop().expect("already checked length");

        match (dec_cspace, enc_cspace) {
            (ColorSpace::Grayscale, ColorSpace::Grayscale) => {
                if subsampling != TRIVIAL_SUBSAMPLING {
                    return Err(Self::Error::general(
                        "Grayscale images cannot have non-trivial subsampling",
                    ));
                }
                Ok(ColorConfig::Grayscale)
            }
            (ColorSpace::Rgb, ColorSpace::Rgb) => {
                if subsampling != TRIVIAL_SUBSAMPLING {
                    return Err(Self::Error::general(
                        "Images stored as RGB cannot have non-trivial subsampling",
                    ));
                }
                Ok(ColorConfig::Rgb)
            }
            (ColorSpace::Rgb, ColorSpace::YCbCr) => Ok(ColorConfig::RgbToYCbCr { subsampling }),
            (ColorSpace::YCbCr, ColorSpace::YCbCr) => Ok(ColorConfig::YCbCr { subsampling }),
            _ => Err(Self::Error::general("Invalid color space combination")),
        }
    }
}

impl From<ColorConfig> for ColorConfigRaw {
    fn from(val: ColorConfig) -> Self {
        match val {
            ColorConfig::Grayscale => ColorConfigRaw {
                encoded_color_space: None,
                decoded_color_space: Some(ColorSpace::Grayscale),
                subsampling: vec![SamplingRatios::default()],
            },
            ColorConfig::Rgb => ColorConfigRaw {
                encoded_color_space: Some(ColorSpace::Rgb),
                decoded_color_space: Some(ColorSpace::Rgb),
                subsampling: vec![SamplingRatios::default(); 3],
            },
            ColorConfig::YCbCr { subsampling } => ColorConfigRaw {
                encoded_color_space: Some(ColorSpace::YCbCr),
                decoded_color_space: Some(ColorSpace::YCbCr),
                subsampling: vec![
                    subsampling,
                    SamplingRatios::default(),
                    SamplingRatios::default(),
                ],
            },
            ColorConfig::RgbToYCbCr { subsampling } => ColorConfigRaw {
                encoded_color_space: Some(ColorSpace::YCbCr),
                decoded_color_space: Some(ColorSpace::Rgb),
                subsampling: vec![
                    subsampling,
                    SamplingRatios::default(),
                    SamplingRatios::default(),
                ],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    use crate::ColorConfig;

    #[test]
    fn test_parse_happy() {
        for (jso, exp) in [
            (
                json!({"decoded_color_space": "rgb", "encoded_color_space": "ycbcr", "subsampling": [[2, 2], [1, 1], [1, 1]]}),
                ColorConfig::RgbToYCbCr {
                    subsampling: SamplingRatios::try_new(2, 2).unwrap(),
                },
            ),
            (
                json!({"decoded_color_space": "rgb", "encoded_color_space": "rgb", "subsampling": [[1, 1], [1, 1], [1, 1]]}),
                ColorConfig::Rgb,
            ),
            (json!({"subsampling": [[1, 1]]}), ColorConfig::Grayscale),
            (
                json!({"encoded_color_space": "grayscale", "subsampling": [[1, 1]]}),
                ColorConfig::Grayscale,
            ),
            (
                json!({"decoded_color_space": "grayscale", "subsampling": [[1, 1]]}),
                ColorConfig::Grayscale,
            ),
        ] {
            let _raw: ColorConfigRaw =
                serde_json::from_value(jso.clone()).expect("should deser raw type");
            let cfg: ColorConfig = serde_json::from_value(jso).expect("should deser parsed type");

            assert_eq!(cfg, exp);
        }
    }

    #[test]
    /// Configurations have the correct fields, but values make them invalid.
    pub fn test_parse_sad() {
        for jso in [
            json!({"decoded_color_space": "rgb", "encoded_color_space": "ycbcr", "subsampling": [[2, 2], [1, 1]]}),
            json!({"decoded_color_space": "rgb", "encoded_color_space": "ycbcr", "subsampling": [[2, 2], [1, 1], [1, 1], [1, 1]]}),
            json!({"decoded_color_space": "grayscale", "encoded_color_space": "ycbcr", "subsampling": [[2, 2]]}),
            json!({"decoded_color_space": "rgb", "encoded_color_space": "grayscale", "subsampling": [[2, 2]]}),
        ] {
            let _raw: ColorConfigRaw =
                serde_json::from_value(jso.clone()).expect("should deser raw type");
            let cfg: Result<ColorConfig, _> = serde_json::from_value(jso);

            assert!(cfg.is_err());
        }
    }
}
