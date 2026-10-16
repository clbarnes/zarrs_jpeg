#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

mod config;
#[cfg(feature = "zarrs")]
mod zarrs_impl;

pub use config::{
    ColorConfig, ColorSpace, JpegCodecConfig, Quality, SamplingRatio, SamplingRatios,
};
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
pub(crate) mod test_utils;
