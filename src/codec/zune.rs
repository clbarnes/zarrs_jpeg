//! Decode-only JPEG implementation in pure rust.
use zune_jpeg::JpegDecoder as ZJpegDecoder;
use zune_jpeg::zune_core::{
    bytestream::ZCursor, colorspace::ColorSpace as ZColorSpace,
    options::DecoderOptions as ZDecoderOptions,
};

use crate::{HasJpegCodecConfig, JpegCodecConfig, JpegDecoderTrait};

/// Decode-only JPEG implementation in pure rust.
pub struct ZuneDecoder {
    config: JpegCodecConfig,
}

impl HasJpegCodecConfig for ZuneDecoder {
    fn config(&self) -> JpegCodecConfig {
        self.config
    }
}

impl From<JpegCodecConfig> for ZuneDecoder {
    fn from(value: JpegCodecConfig) -> Self {
        ZuneDecoder { config: value }
    }
}

impl JpegDecoderTrait for ZuneDecoder {
    fn decode(&self, data: &[u8]) -> crate::Result<(crate::JpegShape, Vec<u8>)> {
        let cursor = ZCursor::new(data);
        let cspace = match self.config.color_config.color_spaces().0 {
            crate::config::ColorSpace::YCbCr => ZColorSpace::YCbCr,
            crate::config::ColorSpace::Rgb => ZColorSpace::RGB,
            crate::config::ColorSpace::Grayscale => ZColorSpace::Luma,
        };
        let opt = ZDecoderOptions::new_cmd().jpeg_set_out_colorspace(cspace);

        let mut decoder = ZJpegDecoder::new_with_options(cursor, opt);
        let buf = decoder.decode()?;
        let (w, h) = decoder.dimensions().expect("just decoded");
        let sh = crate::JpegShape::try_new(w, h)?;
        Ok((sh, buf))
    }
}
