use super::{JpegDecoderTrait, JpegEncoderTrait};
use crate::codec::JpegCodecTrait;
use crate::config::{ColorConfig, JpegCodecConfig, SamplingRatios};
use crate::types::JpegShape;

/// Codec implementation based on libjpeg-turbo bindings.
#[derive(Debug, Clone, Copy)]
pub struct TurboCodec {
    quality: i32,
    decoded_color_space: turbojpeg::Colorspace,
    encoded_color_space: turbojpeg::Colorspace,
    subsampling: turbojpeg::Subsamp,
    mcu_shape: (u16, u16),
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
        let mcu_shape = config.mcu_shape();
        match config.color_config {
            ColorConfig::YCbCr { subsampling } => Ok(TurboCodec {
                quality,
                decoded_color_space: turbojpeg::Colorspace::YCbCr,
                encoded_color_space: turbojpeg::Colorspace::YCbCr,
                subsampling: subsampling.try_into()?,
                mcu_shape,
            }),
            ColorConfig::RgbToYCbCr { subsampling } => Ok(TurboCodec {
                quality,
                decoded_color_space: turbojpeg::Colorspace::RGB,
                encoded_color_space: turbojpeg::Colorspace::YCbCr,
                subsampling: subsampling.try_into()?,
                mcu_shape,
            }),
            ColorConfig::Grayscale => Ok(TurboCodec {
                quality,
                decoded_color_space: turbojpeg::Colorspace::Gray,
                encoded_color_space: turbojpeg::Colorspace::Gray,
                subsampling: turbojpeg::Subsamp::Gray,
                mcu_shape,
            }),
            ColorConfig::Rgb => Ok(TurboCodec {
                quality,
                decoded_color_space: turbojpeg::Colorspace::RGB,
                encoded_color_space: turbojpeg::Colorspace::RGB,
                subsampling: turbojpeg::Subsamp::None,
                mcu_shape,
            }),
        }
    }
}

impl JpegCodecTrait for TurboCodec {
    fn decoded_components(&self) -> usize {
        match self.decoded_color_space {
            turbojpeg::Colorspace::RGB => 3,
            turbojpeg::Colorspace::Gray => 1,
            turbojpeg::Colorspace::YCbCr => 3,
            _ => unreachable!(
                "Unsupported decoded color space {:?}",
                self.decoded_color_space
            ),
        }
    }

    fn mcu_shape(&self) -> (u16, u16) {
        self.mcu_shape
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
