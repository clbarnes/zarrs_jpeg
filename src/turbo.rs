use std::num::NonZeroU16;

use serde::{Deserialize, Serialize};
use turbojpeg::{Colorspace, Compressor, Image, PixelFormat, Subsamp};

use crate::{
    Quality,
    codec::{ChromaSubsampling, ColorConfig, JpegCodecConfiguration, JpegShape},
};

/// Construct using [TryFrom<JpecCodecConfiguration>].
#[derive(Debug, Clone, Copy)]
pub(crate) struct TurboEncoder {
    quality: i32,
    color_space: turbojpeg::Colorspace,
    subsampling: turbojpeg::Subsamp,
}

impl From<turbojpeg::Subsamp> for ChromaSubsampling {
    fn from(val: turbojpeg::Subsamp) -> Self {
        match val {
            Subsamp::None => ChromaSubsampling::Cs4_4_4,
            Subsamp::Sub2x1 => ChromaSubsampling::Cs4_2_2,
            Subsamp::Sub2x2 => ChromaSubsampling::Cs4_2_0,
            Subsamp::Sub1x2 => ChromaSubsampling::Cs4_4_0,
            // Subsamp::Gray => todo!(),
            // Subsamp::Sub4x1 => todo!(),
            // Subsamp::Sub1x4 => todo!(),
            // Subsamp::Unknown => todo!(),
            _ => panic!("Unsupported subsampling value"),
        }
    }
}

impl From<JpegCodecConfiguration> for TurboEncoder {
    fn from(value: JpegCodecConfiguration) -> Self {
        let quality = value.quality.value() as i32;
        let (color_space, subsampling) = match value.color_config {
            ColorConfig::YCbCr { subsampling } => {
                let subsampling = match subsampling {
                    ChromaSubsampling::Cs4_4_4 => Subsamp::None,
                    ChromaSubsampling::Cs4_2_2 => Subsamp::Sub2x1,
                    ChromaSubsampling::Cs4_2_0 => Subsamp::Sub2x2,
                    ChromaSubsampling::Cs4_4_0 => Subsamp::Sub1x2,
                };
                (Colorspace::YCbCr, subsampling)
            }
            ColorConfig::Rgb => (Colorspace::RGB, Subsamp::None),
            ColorConfig::Grayscale => (Colorspace::Gray, Subsamp::None),
        };
        Self {
            quality,
            color_space,
            subsampling,
        }
    }
}

impl TurboEncoder {
    fn make_compressor(&self, shape: &JpegShape) -> Result<Compressor, &'static str> {
        if shape.is_rgb {
            if self.color_space != Colorspace::RGB && self.color_space != Colorspace::YCbCr {
                return Err("RGB input requires RGB or YCbCr color space");
            }
        } else {
            if self.color_space != Colorspace::Gray {
                return Err("Grayscale input requires Gray color space");
            }
        }

        let mut compressor = Compressor::new().map_err(|_| "Failed to create compressor")?;
        compressor
            .set_quality(self.quality)
            .map_err(|_| "Could not set quality")?;
        compressor
            .set_optimize(true)
            .map_err(|_| "Could not set optimize")?;

        compressor
            .set_colorspace(self.color_space)
            .map_err(|_| "Could not set colorspace")?;
        compressor
            .set_subsamp(self.subsampling)
            .map_err(|_| "Could not set subsampling")?;

        Ok(compressor)
    }

    // pub(crate) fn max_encoded_size(&self, shape: &JpegShape) -> Result<usize, &'static str> {
    //     if shape.is_rgb == self.color_config.is_grayscale() {
    //         return Err("Mismatch between shape color space and encoder color space");
    //     }
    //     self.make_compressor(shape)?
    //         .buf_len(shape.width.get() as usize, shape.height.get() as usize)
    //         .map_err(|_| "Failed to calculate buffer length")
    // }

    pub(crate) fn encode(&self, shape: &JpegShape, pixels: &[u8]) -> Result<Vec<u8>, &'static str> {
        let mut compressor = self.make_compressor(shape)?;
        let width = shape.width.get() as usize;
        let (pitch, format) = if shape.is_rgb {
            (width * 3, PixelFormat::RGB)
        } else {
            (width, PixelFormat::GRAY)
        };
        let img = Image {
            pixels,
            width,
            pitch,
            height: shape.height.get() as usize,
            format,
        };
        let out = compressor
            .compress_to_vec(img)
            .map_err(|_| "Failed to compress image")?;
        Ok(out)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct TurboDecoder;

fn try_nzu16<T: TryInto<u16>>(value: T) -> Result<NonZeroU16, &'static str> {
    NonZeroU16::new(value.try_into().map_err(|_| "Value out of range for u16")?)
        .ok_or("Value cannot be zero")
}

impl TurboDecoder {
    pub fn decode(
        &self,
        is_rgb: bool,
        encoded: &[u8],
    ) -> Result<(JpegShape, Vec<u8>), &'static str> {
        let px_fmt = if is_rgb {
            turbojpeg::PixelFormat::RGB
        } else {
            turbojpeg::PixelFormat::GRAY
        };
        let img = turbojpeg::decompress(encoded, px_fmt).unwrap();
        let sh = JpegShape {
            width: try_nzu16(img.width)?,
            height: try_nzu16(img.height)?,
            is_rgb: {
                match img.format {
                    turbojpeg::PixelFormat::RGB => true,
                    turbojpeg::PixelFormat::GRAY => false,
                    _ => return Err("Unsupported pixel format"),
                }
            },
        };
        Ok((sh, img.pixels))
    }
}

#[derive(Debug, Copy, Clone)]
pub(crate) struct TurboCodec {
    pub encoder: TurboEncoder,
    pub decoder: TurboDecoder,
}

impl<'de> Deserialize<'de> for TurboCodec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let config = JpegCodecConfiguration::deserialize(deserializer)?;
        Ok(TurboCodec::from(config))
    }
}

impl Serialize for TurboCodec {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let config = JpegCodecConfiguration::try_from(*self).map_err(serde::ser::Error::custom)?;
        config.serialize(serializer)
    }
}

impl TryFrom<TurboCodec> for JpegCodecConfiguration {
    type Error = &'static str;

    fn try_from(value: TurboCodec) -> Result<Self, Self::Error> {
        let quality = Quality::try_from(value.encoder.quality as u8)?;
        let color_config = match value.encoder.color_space {
            Colorspace::RGB => {
                if value.encoder.subsampling != Subsamp::None {
                    return Err("RGB color space must have no subsampling");
                }
                ColorConfig::Rgb
            }
            Colorspace::Gray => {
                if value.encoder.subsampling != Subsamp::None {
                    return Err("RGB color space must have no subsampling");
                }
                ColorConfig::Grayscale
            }
            Colorspace::YCbCr => {
                let subsampling = ChromaSubsampling::from(value.encoder.subsampling);
                ColorConfig::YCbCr { subsampling }
            }
            _ => return Err("Unsupported color space"),
        };
        Ok(Self {
            quality,
            color_config,
        })
    }
}

impl From<JpegCodecConfiguration> for TurboCodec {
    fn from(value: JpegCodecConfiguration) -> Self {
        let encoder = TurboEncoder::from(value);
        let decoder = TurboDecoder;
        Self { encoder, decoder }
    }
}
