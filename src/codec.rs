use std::{
    num::{NonZeroU16, NonZeroU64},
    sync::Arc,
};

use crate::{
    JpegCodecConfiguration,
    turbo::TurboCodec,
    types::{ColorConfig, JpegShape, Quality},
};
use serde::{Deserialize, Serialize};
use zarrs::array::{
    ArrayBytes, ArrayBytesRaw, ArrayCodecTraits, ArrayToBytesCodecTraits, BytesRepresentation,
    Codec, CodecError, CodecOptions, CodecTraits, CodecTraitsV3, DataType, FillValue,
    IncompatibleDimensionalityError,
    codec::api::{
        CodecPluginV3, ExpectedFixedLengthBytesError, InvalidArrayShapeError,
        PartialDecoderCapability, PartialEncoderCapability,
    },
};

zarrs::plugin::impl_extension_aliases!(JpegCodec, v3: "jpeg", ["zarrs.jpeg"]);
inventory::submit! {CodecPluginV3::new::<JpegCodec>()}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JpegCodec(TurboCodec);

impl JpegCodec {
    pub fn new(quality: Quality, color_config: ColorConfig) -> Self {
        let codec = TurboCodec::from(JpegCodecConfiguration {
            quality,
            color_config,
        });
        Self(codec)
    }
}

impl CodecTraitsV3 for JpegCodec {
    fn create(
        metadata: &zarrs::metadata::v3::MetadataV3,
    ) -> Result<zarrs::array::Codec, zarrs::plugin::PluginCreateError>
    where
        Self: Sized,
    {
        let configuration: JpegCodecConfiguration = metadata.to_typed_configuration()?;
        let codec = Arc::new(JpegCodec(TurboCodec::from(configuration)));
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
        let config =
            JpegCodecConfiguration::try_from(self.0).expect("inner jpeg codec must be valid");
        let val =
            serde_json::to_value(config).expect("jpeg codec configuration should be serializable");
        let serde_json::Value::Object(map) = val else {
            panic!("jpeg codec configuration should serialize to a JSON object");
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
        let sh = JpegShape::try_from(shape)?;
        self.0
            .encoder
            .max_encoded_size(&sh)
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
        let im_shape = JpegShape::try_from(shape)?;
        let b = get_bytes(&bytes)?;
        let out_b = self
            .0
            .encoder
            .encode(&im_shape, b)
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
        let sh = JpegShape::try_from(shape)?;
        let out_b = self.0.decoder.decode(&sh, bytes.as_ref())?;

        Ok(ArrayBytes::new_flen(out_b))
    }
}

impl TryFrom<&[NonZeroU64]> for JpegShape {
    type Error = CodecError;

    fn try_from(shape: &[NonZeroU64]) -> Result<Self, Self::Error> {
        let (is_rgb, w, h) = match shape.len() {
            2 => (false, shape[0], shape[1]),
            3 => match shape[2].get() {
                1 => (false, shape[0], shape[1]),
                3 => (true, shape[0], shape[1]),
                _ => {
                    return Err(InvalidArrayShapeError::new(
                        shape.iter().map(|n| n.get()).collect(),
                        3,
                    )
                    .into());
                }
            },
            n => {
                return Err(IncompatibleDimensionalityError::new(n, 3).into());
            }
        };
        let width: u16 = w.get().try_into().map_err(|_| {
            InvalidArrayShapeError::new(shape.iter().map(|n| n.get()).collect(), u16::MAX as usize)
        })?;
        let height: u16 = h.get().try_into().map_err(|_| {
            InvalidArrayShapeError::new(shape.iter().map(|n| n.get()).collect(), u16::MAX as usize)
        })?;
        Ok(Self {
            width: NonZeroU16::new(width).unwrap(),
            height: NonZeroU16::new(height).unwrap(),
            is_rgb,
        })
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
