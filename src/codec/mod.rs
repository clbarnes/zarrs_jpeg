//! Codec implementations.
use crate::config::JpegCodecConfig;
use crate::types::JpegShape;

#[cfg(feature = "backend-turbojpeg")]
mod turbo;
#[cfg(feature = "backend-turbojpeg")]
pub use turbo::TurboCodec;

#[cfg(feature = "backend-encode-mozjpeg-rs")]
mod moz;
#[cfg(feature = "backend-encode-mozjpeg-rs")]
pub use moz::MozJpegRsEncoder;

#[cfg(feature = "backend-decode-zune-jpeg")]
mod zune;
#[cfg(feature = "backend-decode-zune-jpeg")]
pub use zune::ZuneDecoder;

mod null;
pub use null::{NullDecoder, NullEncoder};

mod split;
pub use split::SplitCodec;

impl HasJpegCodecConfig for JpegCodecConfig {
    fn config(&self) -> JpegCodecConfig {
        *self
    }
}

/// Trait for types which contain a [JpegCodecConfig] and can provide information about the JPEG configuration.
pub trait HasJpegCodecConfig {
    /// Constant factor for estimating the maximum size of an encoded JPEG.
    /// Multiplies the number of stored pixels (accounting for chroma subsampling).
    const ENCODE_BUFFER_FACTOR: usize = 2;
    /// Constant minimum buffer size for encoding, used for estimating output buffer size.
    const ENCODE_BUFFER_CONSTANT: usize = 1024;

    /// Get the associated [JpegCodecConfig].
    fn config(&self) -> JpegCodecConfig;

    /// Get the number of decoded components (channels) for the JPEG configuration.
    fn decoded_components(&self) -> usize {
        self.config().color_config().color_spaces().0.components()
    }

    /// Get the MCU (Minimum Coded Unit) shape for the JPEG configuration.
    fn mcu_shape(&self) -> (u16, u16) {
        self.config().color_config().mcu_shape()
    }

    /// Get the maximum size in bytes of an encoded JPEG.
    /// This may be a very large estimate; the number of stored pixels (accounting for chroma subsampling) multiplied by [Self::ENCODE_BUFFER_FACTOR] plus [Self::ENCODE_BUFFER_CONSTANT].
    ///
    /// This can be used as a fallback for encoders which do not provide their own estimate.
    fn max_encoded_size_fallback(&self, shape: JpegShape) -> usize {
        let n_pixels = shape.width.get() as usize * shape.height.get() as usize;
        let stored_px = match self.config().color_config() {
            crate::config::ColorConfig::Grayscale => n_pixels,
            crate::config::ColorConfig::Rgb => n_pixels * 3,
            crate::config::ColorConfig::YCbCr { subsampling }
            | crate::config::ColorConfig::RgbToYCbCr { subsampling } => {
                n_pixels
                    + n_pixels / subsampling.horizontal.value() as usize
                    + n_pixels / subsampling.vertical.value() as usize
            }
        };
        Self::ENCODE_BUFFER_CONSTANT + stored_px * Self::ENCODE_BUFFER_FACTOR
    }
}

/// Base trait for JPEG codec-like types,
/// which calculates features about the JPEG which this codec requires.
pub trait JpegCodecTraits:
    TryFrom<JpegCodecConfig, Error = crate::Error>
    + HasJpegCodecConfig
    + JpegEncoderTrait
    + JpegDecoderTrait
{
}

/// Trait for encoding JPEG images.
pub trait JpegEncoderTrait: HasJpegCodecConfig {
    /// Get the maximum size in bytes of the encoded JPEG data for an image of the given shape.
    fn max_encoded_size(&self, shape: JpegShape) -> crate::Result<usize>;

    /// Raise an error if the bytes to encode are not the correct length for the configuration.
    ///
    /// Should be called early in the `encode` method.
    fn check_decoded_size(&self, shape: JpegShape, data_len: usize) -> crate::Result<()> {
        let expected_len =
            shape.width.get() as usize * shape.height.get() as usize * self.decoded_components();
        if data_len != expected_len {
            return Err(crate::Error::General(format!(
                "Decoded data length {} does not match expected length {} for shape {:?} and {} components",
                data_len,
                expected_len,
                shape,
                self.decoded_components()
            )));
        }
        Ok(())
    }

    /// Encode the given raw pixel data into JPEG format.
    fn encode(&self, data: &[u8], shape: JpegShape) -> crate::Result<Vec<u8>>;
}

/// Trait for decoding JPEG images.
pub trait JpegDecoderTrait {
    /// Decode the JPEG buffer into raw pixel data.
    fn decode(&self, data: &[u8]) -> crate::Result<(JpegShape, Vec<u8>)>;

    /// Decode the given JPEG data into pixel bytes,
    /// validating the shape.
    fn decode_checked(&self, data: &[u8], shape: JpegShape) -> crate::Result<Vec<u8>> {
        let (decoded_shape, decoded_bytes) = self.decode(data)?;
        if decoded_shape != shape {
            return Err(crate::Error::General(format!(
                "Decoded shape {:?} does not match expected shape {:?}",
                decoded_shape, shape
            )));
        }
        Ok(decoded_bytes)
    }
}
