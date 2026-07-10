pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    General(String),
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
    #[error(transparent)]
    Wrapped(Box<dyn std::error::Error>),
}

impl Error {
    pub fn general(msg: impl Into<String>) -> Self {
        Error::General(msg.into())
    }

    pub fn wrap(error: impl std::error::Error + 'static) -> Self {
        Error::Wrapped(Box::new(error))
    }
}
