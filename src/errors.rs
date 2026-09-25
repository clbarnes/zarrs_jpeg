use crate::JpegShape;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    General(String),
    #[error("{0}")]
    Wrapped(#[from] Box<dyn std::error::Error + Send + Sync>),
    #[error("TurboJPEG error: {0}")]
    Turbo(#[from] turbojpeg::Error),
    #[error("Expected {expected:?}, got {got:?}")]
    UnexpectedShape { expected: JpegShape, got: JpegShape },
}

impl Error {
    pub fn general(msg: impl Into<String>) -> Self {
        Error::General(msg.into())
    }

    pub fn wrap<E: std::error::Error + 'static + Send + Sync>(err: E) -> Self {
        Error::Wrapped(Box::new(err))
    }

    pub fn unexpected_shape(expected: JpegShape, got: JpegShape) -> Self {
        Error::UnexpectedShape { expected, got }
    }
}
