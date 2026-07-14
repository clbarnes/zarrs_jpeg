mod codec;
mod turbo;

pub use codec::{ChromaSubsampling, ColorConfig, JpegCodec, JpegCodecConfiguration, Quality};

pub const DEFAULT_QUALITY: u8 = 90;
