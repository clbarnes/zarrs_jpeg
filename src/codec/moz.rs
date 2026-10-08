use crate::JpegCodecConfig;

#[derive(Debug, Clone)]
pub struct MozJpegRsEncoder {
    config: JpegCodecConfig,
}

impl TryFrom<JpegCodecConfig> for MozJpegRsEncoder {
    type Error = crate::Error;

    fn try_from(value: JpegCodecConfig) -> Result<Self, Self::Error> {
        Ok(MozJpegRsEncoder { config: value })
    }
}
