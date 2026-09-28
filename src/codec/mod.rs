use crate::config::JpegCodecConfig;
use crate::types::JpegShape;

mod turbo;
pub(crate) use turbo::TurboCodec;

/// Base trait for JPEG codec-like types,
/// which calculates features about the JPEG which this codec requires.
pub trait JpegCodecTrait: TryFrom<JpegCodecConfig> {
    /// How many components are in the decoded image (1 for grayscale, 3 for RGB or YCbCr).
    fn decoded_components(&self) -> usize;

    /// Width and height in pixels of the minimum coding unit (MCU) for this configuration.
    fn mcu_shape(&self) -> (u16, u16);
}

/// Trait for encoding JPEG images.
pub trait JpegEncoderTrait: JpegCodecTrait {
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
pub trait JpegDecoderTrait: JpegCodecTrait {
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
