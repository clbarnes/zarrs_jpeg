use std::{num::NonZeroU64, sync::Arc};

use crate::{JpegCodec, JpegCodecConfig};
use crate::{
    codec::{JpegCodecTrait, JpegDecoderTrait, JpegEncoderTrait},
    types::JpegShape,
};
use zarrs::{
    array::{
        ArrayBytes, ArrayBytesRaw, ArrayCodecTraits, ArrayToBytesCodecTraits, BytesRepresentation,
        Codec, CodecError, CodecOptions, CodecTraits, CodecTraitsV3, DataType, FillValue,
        IncompatibleDimensionalityError,
        codec::api::{
            CodecPluginV3, ExpectedFixedLengthBytesError, PartialDecoderCapability,
            PartialEncoderCapability,
        },
    },
    plugin::PluginConfigurationInvalidError,
};

zarrs::plugin::impl_extension_aliases!(JpegCodec, v3: "jpeg", ["zarrs.jpeg"]);
inventory::submit! {CodecPluginV3::new::<JpegCodec>()}

impl CodecTraitsV3 for JpegCodec {
    fn create(
        metadata: &zarrs::metadata::v3::MetadataV3,
    ) -> Result<zarrs::array::Codec, zarrs::plugin::PluginCreateError>
    where
        Self: Sized,
    {
        let configuration: JpegCodecConfig = metadata.to_typed_configuration()?;
        let codec = Arc::new(JpegCodec::try_from(configuration).map_err(|e| {
            zarrs::plugin::PluginCreateError::ConfigurationInvalid(
                PluginConfigurationInvalidError::new(format!("Could not build JPEG codec: {e}")),
            )
        })?);
        Ok(Codec::ArrayToBytes(codec))
    }
}

impl CodecTraits for JpegCodec {
    fn as_any(&self) -> &dyn std::any::Any {
        todo!()
    }

    fn configuration(
        &self,
        version: zarrs::plugin::ZarrVersion,
        _options: &zarrs::array::CodecMetadataOptions,
    ) -> Option<zarrs::metadata::Configuration> {
        if version != zarrs::plugin::ZarrVersion::V3 {
            return None;
        }
        let val = serde_json::to_value(self.config)
            .expect("jpeg codec configuration should be serializable");
        let serde_json::Value::Object(map) = val else {
            unreachable!("jpeg codec configuration should serialize to a JSON object");
        };
        Some(map.into())
    }

    fn partial_decoder_capability(&self) -> zarrs::array::codec::api::PartialDecoderCapability {
        PartialDecoderCapability {
            partial_decode: false,
            partial_read: false,
        }
    }

    fn partial_encoder_capability(&self) -> zarrs::array::codec::api::PartialEncoderCapability {
        PartialEncoderCapability {
            partial_encode: false,
        }
    }
}

impl ArrayCodecTraits for JpegCodec {
    fn recommended_concurrency(
        &self,
        _shape: &[std::num::NonZeroU64],
        _data_type: &zarrs::array::DataType,
    ) -> Result<zarrs::array::RecommendedConcurrency, zarrs::array::CodecError> {
        Ok(zarrs::array::RecommendedConcurrency::new_minimum(1))
    }
}

impl ArrayToBytesCodecTraits for JpegCodec {
    /// Return a dynamic version of the codec.
    fn into_dyn(self: Arc<Self>) -> Arc<dyn ArrayToBytesCodecTraits> {
        self
    }

    /// Returns the size of the encoded representation given a size of the decoded representation.
    ///
    /// # Errors
    /// Returns a [`CodecError`] if the decoded representation is not supported by this codec.
    fn encoded_representation(
        &self,
        shape: &[NonZeroU64],
        data_type: &DataType,
        _fill_value: &FillValue,
    ) -> Result<BytesRepresentation, CodecError> {
        check_dtype(data_type)?;
        let im_shape = get_shape(shape, self.codec.decoded_components())?;
        log_bad_chunk_shape(shape, self.mcu_shape());
        self.codec
            .max_encoded_size(im_shape)
            .map(|s| BytesRepresentation::BoundedSize(s as u64))
            .map_err(|e| CodecError::Other(e.to_string()))
    }

    /// Encode a chunk.
    ///
    /// # Errors
    /// Returns [`CodecError`] if a codec fails or `bytes` is incompatible with the decoded representation.
    fn encode<'a>(
        &self,
        bytes: ArrayBytes<'a>,
        shape: &[NonZeroU64],
        data_type: &DataType,
        _fill_value: &FillValue,
        _options: &CodecOptions,
    ) -> Result<ArrayBytesRaw<'a>, CodecError> {
        check_dtype(data_type)?;
        let im_shape = get_shape(shape, self.codec.decoded_components())?;
        log_bad_chunk_shape(shape, self.mcu_shape());
        let b = get_bytes(&bytes)?;
        let out_b = JpegEncoderTrait::encode(self, b, im_shape)
            .map_err(|e| CodecError::Other(e.to_string()))?;
        Ok(ArrayBytesRaw::Owned(out_b))
    }

    /// Decode a chunk.
    ///
    /// # Errors
    /// Returns [`CodecError`] if a codec fails or the decoded output is incompatible with the decoded representation.
    fn decode<'a>(
        &self,
        bytes: ArrayBytesRaw<'a>,
        shape: &[NonZeroU64],
        data_type: &DataType,
        _fill_value: &FillValue,
        _options: &CodecOptions,
    ) -> Result<ArrayBytes<'a>, CodecError> {
        check_dtype(data_type)?;
        let sh = get_shape(shape, self.codec.decoded_components())?;
        let out_b = self
            .decode_checked(bytes.as_ref(), sh)
            .map_err(|e| CodecError::Other(e.to_string()))?;

        Ok(ArrayBytes::new_flen(out_b))
    }
}

fn convert_shape(w: NonZeroU64, h: NonZeroU64) -> Result<JpegShape, CodecError> {
    JpegShape::try_new(w.get(), h.get())
        .map_err(|e| CodecError::Other(format!("Invalid shape for JPEG: {e}")))
}

fn get_shape(shape: &[NonZeroU64], decoded_components: usize) -> Result<JpegShape, CodecError> {
    match shape.len() {
        2 => {
            if decoded_components != 1 {
                return Err(CodecError::IncompatibleDimensionalityError(
                    IncompatibleDimensionalityError::new(2, 3),
                ));
            }
            convert_shape(shape[1], shape[0])
        }
        3 => {
            if decoded_components != shape[2].get() as usize {
                return Err(CodecError::Other(format!(
                    "Last chunk dimension must be {}, got {}",
                    decoded_components, shape[2]
                )));
            }
            convert_shape(shape[1], shape[0])
        }
        n => {
            let expected = if n < 2 { 2 } else { 3 };
            Err(CodecError::IncompatibleDimensionalityError(
                IncompatibleDimensionalityError::new(n, expected),
            ))
        }
    }
}

fn get_bytes<'a>(array_bytes: &'a ArrayBytes<'a>) -> Result<&'a [u8], CodecError> {
    match array_bytes {
        ArrayBytes::Fixed(bytes) => Ok(bytes.as_ref()),
        _ => Err(ExpectedFixedLengthBytesError.into()),
    }
}

fn check_dtype(data_type: &DataType) -> Result<(), CodecError> {
    if data_type.name_v3().as_deref() != Some("uint8") {
        return Err(CodecError::Other(format!(
            "jpeg codec only supports uint8 data type, but got {:?}",
            data_type
        )));
    }
    Ok(())
}

/// Spam users with warnings if the chunk shape is not a multiple of the MCU shape.
fn log_bad_chunk_shape(shape: &[NonZeroU64], mcu_shape: (u16, u16)) {
    if shape.len() < 2 {
        return;
    }
    let w = shape[1].get();
    let h = shape[0].get();
    let mcu_w = mcu_shape.0 as u64;
    let mcu_h = mcu_shape.1 as u64;
    if !w.is_multiple_of(mcu_w) || !h.is_multiple_of(mcu_h) {
        log::warn!("Chunk shape w={w}, h={h} is not a multiple of MCU shape w={mcu_w}, h={mcu_h}",);
    }
}
