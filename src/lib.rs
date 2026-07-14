mod codec;
mod turbo;
mod types;

pub use codec::JpegCodec;
pub use types::{ChromaSubsampling, ColorConfig, JpegCodecConfiguration, Quality};

pub const DEFAULT_QUALITY: u8 = 90;
