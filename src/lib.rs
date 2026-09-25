#![doc = include_str!("../README.md")]
pub mod config;
#[cfg(feature = "zarrs")]
mod zarrs_impl;

pub use config::{ColorConfig, JpegCodecConfig, SamplingRatio, SamplingRatios};
mod errors;
mod types;
pub use errors::{Error, Result};

pub mod codec;
pub use codec::{JpegCodecTrait, JpegDecoderTrait, JpegEncoderTrait};

use serde::{Deserialize, Serialize};

pub const DEFAULT_QUALITY: u8 = 95;

pub use crate::types::JpegShape;

/// Codec for encoding and decoding JPEG images.
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
