use crate::{HasJpegCodecConfig, JpegCodecConfig, JpegEncoderTrait, SamplingRatios};

/// Implements encoding using the [mozjpeg-rs](https://crates.io/crates/mozjpeg-rs) crate,
/// which is a rust implementation of mozjpeg-like optimisations with no relationship to Mozilla (to my knowledge)
/// and makes heavy use of LLMs in the development and release process.
///
/// Does not yet support direct RGB->RGB encoding, [but will soon](https://github.com/imazen/mozjpeg-rs/pull/11).
/// Does not yet support accurate estimates of maximum encoded buffer size, [but may at some point](https://github.com/imazen/mozjpeg-rs/issues/12).
/// YCbCr->YCbCr encoding is supported but incurs an extra clone to convert interleaved to planar data.
#[derive(Debug, Clone)]
pub struct MozJpegRsEncoder {
    config: JpegCodecConfig,
    encoder: mozjpeg_rs::Encoder,
}

impl HasJpegCodecConfig for MozJpegRsEncoder {
    fn config(&self) -> JpegCodecConfig {
        self.config
    }
}

impl TryFrom<SamplingRatios> for mozjpeg_rs::Subsampling {
    type Error = crate::Error;

    fn try_from(value: SamplingRatios) -> Result<Self, Self::Error> {
        let jab = value.as_jab_triple().ok_or_else(|| {
            crate::Error::general(format!("unsupported subsampling ratio {value:?}"))
        })?;
        let out = match jab {
            (4, 4, 4) => mozjpeg_rs::Subsampling::S444,
            (4, 2, 2) => mozjpeg_rs::Subsampling::S422,
            (4, 2, 0) => mozjpeg_rs::Subsampling::S420,
            (4, 4, 0) => mozjpeg_rs::Subsampling::S440,
            _ => {
                return Err(crate::Error::general(format!(
                    "unsupported subsampling ratio {value:?}"
                )));
            }
        };
        Ok(out)
    }
}

impl TryFrom<JpegCodecConfig> for MozJpegRsEncoder {
    type Error = crate::Error;

    fn try_from(value: JpegCodecConfig) -> Result<Self, Self::Error> {
        let subsampl = match value.color_config() {
            crate::ColorConfig::Grayscale => mozjpeg_rs::Subsampling::Gray,
            crate::ColorConfig::Rgb => {
                // available from next release, see https://github.com/imazen/mozjpeg-rs/pull/11
                // mozjpeg_rs::Subsampling::S444
                return Err(crate::Error::general(
                    "RGB encoded color config is not supported by mozjpeg_rs encoder",
                ));
            }
            crate::ColorConfig::YCbCr { subsampling }
            | crate::ColorConfig::RgbToYCbCr { subsampling } => subsampling.try_into()?,
        };
        let encoder = mozjpeg_rs::Encoder::fastest()
            .baseline(true)
            .quality(value.quality())
            .optimize_huffman(true)
            .chroma_subsampling(subsampl);
        Ok(MozJpegRsEncoder {
            config: value,
            encoder,
        })
    }
}

/// Panics if length of
fn interleaved_to_planar(interleaved_data: &[u8], channels: usize) -> crate::Result<Vec<u8>> {
    let dlen = interleaved_data.len();

    let mut out = vec![0u8; dlen];
    if !dlen.is_multiple_of(channels) {
        return Err(crate::Error::general(format!(
            "interleaved data length ({dlen}) must be a multiple of the number of channels ({channels})"
        )));
    }
    let channel_offset = dlen / channels;
    for (idx, chunk) in interleaved_data.chunks(channels).enumerate() {
        for (channel_idx, &value) in chunk.iter().enumerate() {
            let out_idx = channel_idx * channel_offset + idx;
            out[out_idx] = value;
        }
    }
    Ok(out)
}

impl JpegEncoderTrait for MozJpegRsEncoder {
    fn encode(&self, data: &[u8], shape: crate::JpegShape) -> crate::Result<Vec<u8>> {
        self.check_decoded_size(shape, data.len())?;
        let width = shape.width.get().into();
        let height = shape.height.get().into();
        let out = match self.config.color_config().color_spaces().0 {
            crate::config::ColorSpace::Grayscale => self.encoder.encode_gray(data, width, height),
            crate::config::ColorSpace::Rgb => self.encoder.encode_rgb(data, width, height),
            crate::config::ColorSpace::YCbCr => {
                let planar = interleaved_to_planar(data, 3)?;
                let n_pixels = data.len() / 3;
                self.encoder.encode_ycbcr_planar(
                    &planar[..n_pixels],
                    &planar[n_pixels..2 * n_pixels],
                    &planar[2 * n_pixels..],
                    width,
                    height,
                )
            }
        };
        out.map_err(Into::into)
    }

    fn max_encoded_size(&self, shape: crate::JpegShape) -> crate::Result<usize> {
        Ok(self.max_encoded_size_fallback(shape))
    }
}
