use thiserror::Error;
use tokio::sync::watch::error::{RecvError, SendError};

#[derive(Debug, Error)]
pub enum UrlError {
    #[error("Io Error: {0}")]
    Generic(#[from] std::io::Error),
    #[error("Request Error: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Invalid Url")]
    InvalidUrl,
}

#[derive(Debug, Error)]
pub enum File2DlError {
    #[error("Io Error: {0}")]
    Generic(#[from] std::io::Error),
    #[error("Request Error: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Watch Channel Reception Failed: {0}")]
    ChannelRecvError(#[from] RecvError),
    #[error("Watch Channel Sending Failed: {0}")]
    ChannelSendError(#[from] SendError<bool>),
}
