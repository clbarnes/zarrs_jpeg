#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

mod config;
#[cfg(feature = "zarrs")]
mod zarrs_impl;

pub use config::{ColorConfig, JpegCodecConfig, Quality, SamplingRatio, SamplingRatios};
mod errors;
mod types;
pub use errors::{Error, Result};

mod codec;
pub use codec::{JpegCodecTrait, JpegDecoderTrait, JpegEncoderTrait};

use serde::{Deserialize, Serialize};

/// Default quality for JPEG encoding, used when no quality is specified.
pub const DEFAULT_QUALITY: u8 = 95;

pub use crate::types::JpegShape;

/// Codec for encoding and decoding JPEG images.
///
/// Instantiate with [TryFrom] [JpegCodecConfig].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(try_from = "JpegCodecConfig", into = "JpegCodecConfig")]
pub struct JpegCodec {
    config: JpegCodecConfig,
    codec: codec::TurboCodec,
}

impl JpegCodec {
    /// Get the configuration object for this codec.
    pub fn config(&self) -> &JpegCodecConfig {
        &self.config
    }
}

impl JpegCodecTrait for JpegCodec {
    fn decoded_components(&self) -> usize {
        self.codec.decoded_components()
    }

    fn mcu_shape(&self) -> (u16, u16) {
        self.codec.mcu_shape()
    }
}

impl JpegEncoderTrait for JpegCodec {
    fn max_encoded_size(&self, shape: JpegShape) -> crate::Result<usize> {
        self.codec.max_encoded_size(shape)
    }

    fn encode(&self, data: &[u8], shape: JpegShape) -> crate::Result<Vec<u8>> {
        self.codec.encode(data, shape)
    }
}

impl JpegDecoderTrait for JpegCodec {
    fn decode(&self, data: &[u8]) -> crate::Result<(JpegShape, Vec<u8>)> {
        self.codec.decode(data)
    }

    fn decode_checked(&self, data: &[u8], shape: JpegShape) -> crate::Result<Vec<u8>> {
        self.codec.decode_checked(data, shape)
    }
}

impl From<JpegCodec> for JpegCodecConfig {
    fn from(codec: JpegCodec) -> Self {
        codec.config
    }
}

impl TryFrom<JpegCodecConfig> for JpegCodec {
    type Error = crate::Error;

    fn try_from(config: JpegCodecConfig) -> Result<Self, Self::Error> {
        let codec = codec::TurboCodec::try_from(config)?;
        Ok(Self { config, codec })
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::{
        io::{BufReader, Cursor},
        sync::LazyLock,
    };

    use super::*;

    pub const PNG_PX: &[u8] = include_bytes!("../data/input/astronaut.png");

    pub static RAW_IMG: LazyLock<(JpegShape, Vec<u8>)> = LazyLock::new(read_png_px);
    pub static RAW_IMG_GRAYSCALES: LazyLock<(JpegShape, [Vec<u8>; 3])> = LazyLock::new(|| {
        let (shape, px) = RAW_IMG.clone();
        let (r, g, b) = px.as_chunks::<3>().0.iter().fold(
            (Vec::new(), Vec::new(), Vec::new()),
            |mut acc, chunk| {
                acc.0.push(chunk[0]);
                acc.1.push(chunk[1]);
                acc.2.push(chunk[2]);
                acc
            },
        );
        (shape, [r, g, b])
    });

    /// Read the RGB PNG image
    fn read_png_px() -> (JpegShape, Vec<u8>) {
        let decoder = png::Decoder::new(BufReader::new(Cursor::new(PNG_PX)));
        let mut reader = decoder.read_info().unwrap();

        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buf).unwrap();
        assert_eq!(info.color_type, png::ColorType::Rgb);
        assert_eq!(info.bit_depth, png::BitDepth::Eight);
        let buf = &buf[..info.buffer_size()];
        (
            JpegShape::try_new(info.width, info.height).unwrap(),
            buf.to_vec(),
        )
    }

    pub fn check_codec_roundtrip<C: JpegCodecTrait + JpegDecoderTrait + JpegEncoderTrait>(
        codec: C,
        epsilon: f32,
        px: &[u8],
        shape: JpegShape,
    ) {
        let encoded = codec.encode(px, shape).unwrap();
        let (decoded_shape, decoded_px) = codec.decode(&encoded).unwrap();

        assert_eq!(shape, decoded_shape);
        assert_eq!(px.len(), decoded_px.len());
        for (orig, roundtripped) in px.iter().zip(decoded_px.iter()) {
            let diff = (*orig as f32 - *roundtripped as f32).abs() / u8::MAX as f32;
            assert!(
                diff <= epsilon,
                "Pixel value differs by more than {}% of range: {} vs {}",
                epsilon * 100.0,
                orig,
                roundtripped
            );
        }
    }
}
