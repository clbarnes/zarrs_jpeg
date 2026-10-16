//! Codec composed of separate encoder and decoder implementations.
use super::{HasJpegCodecConfig, JpegCodecTraits, JpegDecoderTrait, JpegEncoderTrait};
use crate::config::JpegCodecConfig;
use crate::types::JpegShape;

/// Codec composed of separate encoder and decoder implementations.
#[derive(Debug, Clone, Copy)]
pub struct SplitCodec<E: JpegEncoderTrait, D: JpegDecoderTrait> {
    encoder: E,
    decoder: D,
}

impl<E: JpegEncoderTrait, D: JpegDecoderTrait> HasJpegCodecConfig for SplitCodec<E, D> {
    fn config(&self) -> JpegCodecConfig {
        self.encoder.config()
    }
}

impl<E: JpegEncoderTrait, D: JpegDecoderTrait> JpegEncoderTrait for SplitCodec<E, D> {
    fn max_encoded_size(&self, shape: JpegShape) -> crate::Result<usize> {
        self.encoder.max_encoded_size(shape)
    }

    fn encode(&self, data: &[u8], shape: JpegShape) -> crate::Result<Vec<u8>> {
        self.encoder.encode(data, shape)
    }
}

impl<E: JpegEncoderTrait, D: JpegDecoderTrait> JpegDecoderTrait for SplitCodec<E, D> {
    fn decode(&self, data: &[u8]) -> crate::Result<(JpegShape, Vec<u8>)> {
        self.decoder.decode(data)
    }
}

impl<E, EErr, D, DErr> TryFrom<JpegCodecConfig> for SplitCodec<E, D>
where
    EErr: Into<crate::Error> + std::error::Error,
    DErr: Into<crate::Error> + std::error::Error,
    E: JpegEncoderTrait + TryFrom<JpegCodecConfig, Error = EErr>,
    D: JpegDecoderTrait + TryFrom<JpegCodecConfig, Error = DErr>,
{
    type Error = crate::Error;

    fn try_from(value: JpegCodecConfig) -> Result<Self, Self::Error> {
        let encoder = E::try_from(value).map_err(Into::into)?;
        let decoder = D::try_from(value).map_err(Into::into)?;
        Ok(SplitCodec { encoder, decoder })
    }
}

impl<E, EErr, D, DErr> JpegCodecTraits for SplitCodec<E, D>
where
    EErr: Into<crate::Error> + std::error::Error,
    DErr: Into<crate::Error> + std::error::Error,
    E: JpegEncoderTrait + TryFrom<JpegCodecConfig, Error = EErr>,
    D: JpegDecoderTrait + TryFrom<JpegCodecConfig, Error = DErr>,
{
}

#[cfg(test)]
mod tests {
    #[cfg(all(
        feature = "backend-encode-mozjpeg-rs",
        feature = "backend-decode-zune-jpeg"
    ))]
    crate::test_utils::impl_roundtrip_test!(crate::DefaultSplitCodec, skip = [rgb]);
}
