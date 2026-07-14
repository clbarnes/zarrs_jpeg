use std::{
    num::{NonZeroU16, NonZeroU64},
    sync::Arc,
};

use crate::{DEFAULT_QUALITY, turbo::TurboCodec};
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

/// Configuration for the JPEG codec.
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct JpegCodecConfiguration {
    pub quality: Quality,
    pub color_config: ColorConfig,
}

impl Serialize for JpegCodecConfiguration {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let raw = JpegCodecConfigurationRaw::from(*self);
        raw.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for JpegCodecConfiguration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = JpegCodecConfigurationRaw::deserialize(deserializer)?;
        JpegCodecConfiguration::try_from(raw).map_err(serde::de::Error::custom)
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
        JpegShape::try_from(shape)?;
        let sz: u64 = shape.iter().map(|s| s.get()).product();

        // Smallest valid JPEG plus number of pixels.
        // Strictly, JPEGs can be any size because you can pack arbitrary app data into them.
        // https://web.archive.org/web/20111224041840/http://www.techsupportteam.org/forum/digital-imaging-photography/1892-worlds-smallest-valid-jpeg.html
        Ok(BytesRepresentation::BoundedSize(134 + sz))
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
        let (out_sh, out_b) = self.0.decoder.decode(sh.is_rgb, bytes.as_ref())?;
        if out_sh != sh {
            return Err(CodecError::Other(format!(
                "Decoded shape {:?} does not match expected shape {:?}",
                out_sh, sh
            )));
        }

        Ok(ArrayBytes::new_flen(out_b))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ColorSpace {
    #[default]
    YCbCr,
    Rgb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromaSubsampling {
    /// No subsampling, 1 chrominance sample per pixel.
    Cs4_4_4,
    /// 2x1 (horizontal) chrominance subsampling; 1 chrominance sample for every 2x1 block of pixels.
    Cs4_2_2,
    /// 2x2 (square) chrominance subsampling; 1 chrominance sample for every 2x2 block of pixels.
    Cs4_2_0,
    /// 1x2 (vertical) chrominance subsampling; 1 chrominance sample for every 1x2 block of pixels.
    /// May incur a performance cost.
    Cs4_4_0,
    // Cs4_4_1,
    // Cs4_1_1,
}

type ChomaSubSamplingArray = [[u8; 2]; 3];

impl ChromaSubsampling {
    pub fn none() -> Self {
        ChromaSubsampling::Cs4_4_4
    }

    pub fn default_ycbcr() -> Self {
        ChromaSubsampling::Cs4_2_0
    }

    fn to_array(self) -> ChomaSubSamplingArray {
        match self {
            ChromaSubsampling::Cs4_4_4 => [[1, 1], [1, 1], [1, 1]],
            ChromaSubsampling::Cs4_2_2 => [[2, 1], [1, 1], [1, 1]],
            ChromaSubsampling::Cs4_2_0 => [[2, 2], [1, 1], [1, 1]],
            ChromaSubsampling::Cs4_4_0 => [[1, 2], [1, 1], [1, 1]],
            // ChromaSubsampling::Cs4_4_1 => [[1, 4], [1, 1], [1, 1]],
            // ChromaSubsampling::Cs4_1_1 => [[4, 4], [1, 1], [1, 1]],
        }
    }

    /// None if not valid.
    fn from_array(arr: &ChomaSubSamplingArray) -> Option<Self> {
        match arr {
            [[1, 1], [1, 1], [1, 1]] => Some(ChromaSubsampling::Cs4_4_4),
            [[2, 1], [1, 1], [1, 1]] => Some(ChromaSubsampling::Cs4_2_2),
            [[2, 2], [1, 1], [1, 1]] => Some(ChromaSubsampling::Cs4_2_0),
            [[1, 2], [1, 1], [1, 1]] => Some(ChromaSubsampling::Cs4_4_0),
            // [[1, 4], [1, 1], [1, 1]] => Some(ChromaSubsampling::Cs4_4_1),
            // [[4, 4], [1, 1], [1, 1]] => Some(ChromaSubsampling::Cs4_1_1),
            _ => None,
        }
    }
}

impl Serialize for ChromaSubsampling {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let arr = self.to_array();
        arr.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ChromaSubsampling {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let arr = ChomaSubSamplingArray::deserialize(deserializer)?;
        Self::from_array(&arr).ok_or_else(|| {
            serde::de::Error::custom(format!("invalid chroma subsampling array: {:?}", arr))
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct JpegShape {
    pub width: NonZeroU16,
    pub height: NonZeroU16,
    pub is_rgb: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorConfig {
    YCbCr { subsampling: ChromaSubsampling },
    Rgb,
    Grayscale,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct ColorConfigRaw {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    encoded_color_space: Option<ColorSpace>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    subsampling: Option<ChromaSubsampling>,
}

impl TryFrom<ColorConfigRaw> for ColorConfig {
    type Error = &'static str;

    fn try_from(value: ColorConfigRaw) -> Result<Self, Self::Error> {
        match value.encoded_color_space {
            Some(ColorSpace::YCbCr) => {
                let subsampling = value.subsampling.unwrap_or(ChromaSubsampling::Cs4_2_0);
                Ok(ColorConfig::YCbCr { subsampling })
            }
            Some(ColorSpace::Rgb) => {
                if let Some(subsampling) = value.subsampling
                    && subsampling != ChromaSubsampling::Cs4_4_4
                {
                    return Err("Invalid subsampling for RGB");
                }
                Ok(ColorConfig::Rgb)
            }
            None => {
                if let Some(subsampling) = value.subsampling
                    && subsampling != ChromaSubsampling::Cs4_4_4
                {
                    return Err("Invalid subsampling for grayscale");
                }
                Ok(ColorConfig::Grayscale)
            }
        }
    }
}

impl From<ColorConfig> for ColorConfigRaw {
    fn from(value: ColorConfig) -> Self {
        match value {
            ColorConfig::YCbCr { subsampling: ss } => {
                // compact representation where possible
                let subsampling = if ss == ChromaSubsampling::default_ycbcr() {
                    None
                } else {
                    Some(ss)
                };
                ColorConfigRaw {
                    encoded_color_space: Some(ColorSpace::YCbCr),
                    subsampling,
                }
            }
            ColorConfig::Rgb => ColorConfigRaw {
                encoded_color_space: Some(ColorSpace::Rgb),
                subsampling: None,
            },
            ColorConfig::Grayscale => ColorConfigRaw {
                encoded_color_space: None,
                subsampling: None,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct JpegCodecConfigurationRaw {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    quality: Option<u8>,
    #[serde(flatten)]
    color_config: ColorConfigRaw,
}

impl TryFrom<JpegCodecConfigurationRaw> for JpegCodecConfiguration {
    type Error = &'static str;

    fn try_from(value: JpegCodecConfigurationRaw) -> Result<Self, Self::Error> {
        let color_config = ColorConfig::try_from(value.color_config)?;
        Ok(JpegCodecConfiguration {
            quality: Quality::try_new(value.quality)?,
            color_config,
        })
    }
}

impl From<JpegCodecConfiguration> for JpegCodecConfigurationRaw {
    fn from(value: JpegCodecConfiguration) -> Self {
        // compact representation where possible
        let quality = if value.quality == Quality::default() {
            None
        } else {
            Some(value.quality.into())
        };
        JpegCodecConfigurationRaw {
            quality,
            color_config: value.color_config.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq)]
pub struct Quality(u8);

impl Default for Quality {
    fn default() -> Self {
        Quality(DEFAULT_QUALITY)
    }
}

impl Quality {
    pub fn max() -> Self {
        Quality(100)
    }

    pub fn try_new(quality: Option<u8>) -> Result<Self, &'static str> {
        if let Some(q) = quality {
            Self::try_from(q)
        } else {
            Ok(Quality(DEFAULT_QUALITY))
        }
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

impl TryFrom<u8> for Quality {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value > 100 {
            return Err("Quality must be between 1 and 100");
        }
        Ok(Quality(value))
    }
}

impl From<Quality> for u8 {
    fn from(value: Quality) -> Self {
        value.0
    }
}
