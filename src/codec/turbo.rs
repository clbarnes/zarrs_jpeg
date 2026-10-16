//! jpeg codec implementation based on libjpeg-turbo bindings.
use super::{JpegDecoderTrait, JpegEncoderTrait};
use crate::codec::{HasJpegCodecConfig, JpegCodecTraits};
use crate::config::{ColorConfig, JpegCodecConfig, SamplingRatios};
use crate::types::JpegShape;

/// Codec implementation based on libjpeg-turbo bindings.
#[derive(Debug, Clone, Copy)]
pub struct TurboCodec {
    config: JpegCodecConfig,
    quality: i32,
    decoded_color_space: turbojpeg::Colorspace,
    encoded_color_space: turbojpeg::Colorspace,
    subsampling: turbojpeg::Subsamp,
}

impl TryFrom<SamplingRatios> for turbojpeg::Subsamp {
    type Error = crate::Error;

    fn try_from(value: SamplingRatios) -> Result<Self, Self::Error> {
        match value.values() {
            (1, 1) => Ok(turbojpeg::Subsamp::None),
            (2, 1) => Ok(turbojpeg::Subsamp::Sub2x1),
            (1, 2) => Ok(turbojpeg::Subsamp::Sub1x2),
            (2, 2) => Ok(turbojpeg::Subsamp::Sub2x2),
            (1, 4) => Ok(turbojpeg::Subsamp::Sub1x4),
            (4, 1) => Ok(turbojpeg::Subsamp::Sub4x1),
            _ => Err(crate::Error::general("Unsupported subsampling ratios")),
        }
    }
}

impl TryFrom<JpegCodecConfig> for TurboCodec {
    type Error = crate::Error;

    fn try_from(config: JpegCodecConfig) -> Result<Self, Self::Error> {
        let quality = config.quality.value() as i32;
        let (decoded_color_space, encoded_color_space, subsampling) = match config.color_config {
            ColorConfig::YCbCr { subsampling } => (
                turbojpeg::Colorspace::YCbCr,
                turbojpeg::Colorspace::YCbCr,
                subsampling.try_into()?,
            ),
            ColorConfig::RgbToYCbCr { subsampling } => (
                turbojpeg::Colorspace::RGB,
                turbojpeg::Colorspace::YCbCr,
                subsampling.try_into()?,
            ),
            ColorConfig::Grayscale => (
                turbojpeg::Colorspace::Gray,
                turbojpeg::Colorspace::Gray,
                turbojpeg::Subsamp::Gray,
            ),
            ColorConfig::Rgb => (
                turbojpeg::Colorspace::RGB,
                turbojpeg::Colorspace::RGB,
                turbojpeg::Subsamp::None,
            ),
        };
        Ok(TurboCodec {
            config,
            quality,
            decoded_color_space,
            encoded_color_space,
            subsampling,
        })
    }
}

impl HasJpegCodecConfig for TurboCodec {
    fn config(&self) -> JpegCodecConfig {
        self.config
    }
}

impl JpegEncoderTrait for TurboCodec {
    fn max_encoded_size(&self, shape: JpegShape) -> crate::Result<usize> {
        turbojpeg::compressed_buf_len(
            shape.width.get() as usize,
            shape.height.get() as usize,
            self.subsampling,
        )
        .map_err(Into::into)
    }

    fn encode(&self, data: &[u8], shape: crate::types::JpegShape) -> crate::Result<Vec<u8>> {
        self.check_decoded_size(shape, data.len())?;

        let mut comp = turbojpeg::Compressor::new()?;
        comp.set_quality(self.quality)?;
        comp.set_optimize(true)?;
        comp.set_colorspace(self.encoded_color_space)?;
        comp.set_subsamp(self.subsampling)?;

        let width = shape.width.get() as usize;
        let height = shape.height.get() as usize;

        match self.decoded_color_space {
            turbojpeg::Colorspace::RGB => {
                let im = turbojpeg::Image {
                    pixels: data,
                    width,
                    pitch: width * 3,
                    height,
                    format: turbojpeg::PixelFormat::RGB,
                };
                comp.compress_to_vec(im).map_err(Into::into)
            }
            turbojpeg::Colorspace::Gray => {
                let im = turbojpeg::Image {
                    pixels: data,
                    width,
                    pitch: width,
                    height,
                    format: turbojpeg::PixelFormat::GRAY,
                };
                comp.compress_to_vec(im).map_err(Into::into)
            }
            turbojpeg::Colorspace::YCbCr => {
                let im = turbojpeg::YuvImage {
                    pixels: data,
                    width,
                    align: 1,
                    height,
                    // This refers to the subsampling present in the decoded pixels
                    subsamp: turbojpeg::Subsamp::None,
                };
                comp.compress_yuv_to_vec(im).map_err(Into::into)
            }
            c => Err(crate::Error::general(format!(
                "Unsupported decoded color space {c:?}"
            ))),
        }
    }
}

impl JpegDecoderTrait for TurboCodec {
    fn decode(&self, data: &[u8]) -> crate::Result<(JpegShape, Vec<u8>)> {
        match self.decoded_color_space {
            turbojpeg::Colorspace::RGB => {
                let px_fmt = turbojpeg::PixelFormat::RGB;
                let img = turbojpeg::decompress(data, px_fmt)?;
                let sh = JpegShape::try_new(img.width, img.height)?;
                Ok((sh, img.pixels))
            }
            turbojpeg::Colorspace::Gray => {
                let px_fmt = turbojpeg::PixelFormat::GRAY;
                let img = turbojpeg::decompress(data, px_fmt)?;
                let sh = JpegShape::try_new(img.width, img.height)?;
                Ok((sh, img.pixels))
            }
            turbojpeg::Colorspace::YCbCr => {
                let img = turbojpeg::decompress_to_yuv(data)?;
                let sh = JpegShape::try_new(img.width, img.height)?;
                Ok((sh, img.pixels))
            }
            c => Err(crate::Error::general(format!(
                "Unsupported decoded color space {c:?}"
            ))),
        }
    }
}

impl JpegCodecTraits for TurboCodec {}

#[cfg(test)]
mod tests {
    use super::TurboCodec;
    use crate::tests::{RAW_IMG, RAW_IMG_GRAYSCALES, check_codec_roundtrip};
    use crate::{ColorConfig, JpegCodecConfig, Quality};

    fn check_config_roundtrip(config: JpegCodecConfig, epsilon: f32) {
        let codec = TurboCodec::try_from(config).unwrap();
        let (shape, px) = RAW_IMG.clone();
        check_codec_roundtrip(codec, epsilon, &px, shape);
    }

    #[test]
    fn test_rgb() {
        let config = JpegCodecConfig::new(Quality::max(), ColorConfig::Rgb);
        check_config_roundtrip(config, 0.1);
    }

    #[test]
    fn test_ycbcr() {
        let config = JpegCodecConfig::new(
            Quality::max(),
            ColorConfig::try_new_rgb_to_ycbcr(1, 1).unwrap(),
        );
        check_config_roundtrip(config, 0.1);
    }

    #[test]
    fn test_ycbcr_subsampled_22() {
        let config = JpegCodecConfig::new(
            Quality::max(),
            ColorConfig::try_new_rgb_to_ycbcr(2, 2).unwrap(),
        );
        check_config_roundtrip(config, 0.2);
    }

    #[test]
    fn test_ycbcr_subsampled_41() {
        let config = JpegCodecConfig::new(
            Quality::max(),
            ColorConfig::try_new_rgb_to_ycbcr(4, 1).unwrap(),
        );
        check_config_roundtrip(config, 0.3);
    }

    #[test]
    fn test_rgb_low_qual() {
        let config = JpegCodecConfig::new(Quality::try_from(50).unwrap(), ColorConfig::Rgb);
        check_config_roundtrip(config, 0.3);
    }

    #[test]
    fn test_grayscale() {
        let config = JpegCodecConfig::new(Quality::max(), ColorConfig::Grayscale);
        let epsilon = 0.1;

        let codec = TurboCodec::try_from(config).unwrap();
        let (shape, [r, _g, _b]) = RAW_IMG_GRAYSCALES.clone();
        check_codec_roundtrip(codec, epsilon, &r, shape);
    }
}
