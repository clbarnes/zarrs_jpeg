use crate::{HasJpegCodecConfig, JpegCodecConfig, JpegDecoderTrait, JpegEncoderTrait};

/// No-op JPEG decoder which always returns an error.
/// Useful with [crate::codec::SplitCodec] for encode-only implementations.
#[derive(Debug, Clone, Copy)]
pub struct NullDecoder {
    config: JpegCodecConfig,
}

impl TryFrom<JpegCodecConfig> for NullDecoder {
    type Error = crate::Error;

    fn try_from(value: JpegCodecConfig) -> Result<Self, Self::Error> {
        Ok(NullDecoder { config: value })
    }
}

impl HasJpegCodecConfig for NullDecoder {
    fn config(&self) -> JpegCodecConfig {
        self.config
    }
}

impl JpegDecoderTrait for NullDecoder {
    fn decode(&self, _data: &[u8]) -> crate::Result<(crate::JpegShape, Vec<u8>)> {
        Err(crate::Error::Unsupported("decode"))
    }
}

/// No-op JPEG encoder which always returns an error.
/// Useful with [crate::codec::SplitCodec] for decode-only implementations.
#[derive(Debug, Clone, Copy)]
pub struct NullEncoder {
    config: JpegCodecConfig,
}

impl TryFrom<JpegCodecConfig> for NullEncoder {
    type Error = crate::Error;

    fn try_from(value: JpegCodecConfig) -> Result<Self, Self::Error> {
        Ok(NullEncoder { config: value })
    }
}

impl HasJpegCodecConfig for NullEncoder {
    fn config(&self) -> JpegCodecConfig {
        self.config
    }
}

impl JpegEncoderTrait for NullEncoder {
    fn max_encoded_size(&self, shape: crate::JpegShape) -> crate::Result<usize> {
        Ok(self.config.max_encoded_size_fallback(shape))
    }

    fn encode(&self, _data: &[u8], _shape: crate::JpegShape) -> crate::Result<Vec<u8>> {
        Err(crate::Error::Unsupported("encode"))
    }
}
