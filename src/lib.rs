#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

mod config;
#[cfg(feature = "zarrs")]
mod zarrs_impl;

pub use config::{ColorConfig, JpegCodecConfig, Quality, SamplingRatio, SamplingRatios};
mod errors;
mod types;
pub use errors::{Error, Result};

pub mod codec;
pub use codec::{HasJpegCodecConfig, JpegDecoderTrait, JpegEncoderTrait};

/// Default quality for JPEG encoding, used when no quality is specified.
pub const DEFAULT_QUALITY: u8 = 95;

pub use crate::types::JpegShape;

/// Codec implementation which delegates to the best available split encode/decode implementation.
pub type DefaultSplitCodec = codec::SplitCodec<
    cfg_select! {
        feature = "backend-encode-mozjpeg-rs" => codec::MozJpegRsEncoder,
        _ => codec::NullEncoder,
    },
    cfg_select! {
        feature = "backend-decode-zune-jpeg" => codec::ZuneDecoder,
        _ => codec::NullDecoder,
    },
>;

/// Preferred codec implementation depending on available features.
pub type JpegCodec = cfg_select! {
    feature = "backend-turbojpeg" => codec::TurboCodec,
    _ => DefaultSplitCodec,
};

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

    pub fn check_codec_roundtrip<C: HasJpegCodecConfig + JpegDecoderTrait + JpegEncoderTrait>(
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
