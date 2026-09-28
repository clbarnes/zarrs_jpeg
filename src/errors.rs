use crate::JpegShape;

/// Result type for `zarrs_jpeg`, using the crate's [Error] type by default.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Error type for `zarrs_jpeg`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// General error with a message.
    #[error("{0}")]
    General(String),
    #[error("{0}")]
    /// Wrapped error from another source.
    Wrapped(#[from] Box<dyn std::error::Error + Send + Sync>),
    /// Error from the TurboJPEG library.
    #[error("TurboJPEG error: {0}")]
    Turbo(#[from] turbojpeg::Error),
    /// Error indicating that the shape of the JPEG image was not as expected.
    #[error("Expected {expected:?}, got {got:?}")]
    UnexpectedShape {
        /// The expected shape of the JPEG image.
        expected: JpegShape,
        /// The actual shape of the JPEG image.
        got: JpegShape,
    },
}

impl Error {
    /// Create a general error from a message.
    pub fn general(msg: impl Into<String>) -> Self {
        Error::General(msg.into())
    }

    /// Wrap an error from another source.
    pub fn wrap<E: std::error::Error + 'static + Send + Sync>(err: E) -> Self {
        Error::Wrapped(Box::new(err))
    }

    /// Create an error indicating that the shape of the JPEG image was not as expected.
    pub fn unexpected_shape(expected: JpegShape, got: JpegShape) -> Self {
        Error::UnexpectedShape { expected, got }
    }
}
