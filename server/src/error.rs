use tokio::sync::mpsc;

use common::error::ConfigError;
use common::signal::Signal;

use crate::config::Config;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Server Already Started, with config: {0:#?}")]
    AlreadyStarted(Config),

    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("WebSocket Error: {0}")]
    WebSocket(#[from] tungstenite::Error),

    #[error("Udp Error: {0}")]
    Udp(String),

    #[error("Channel Error: {0}")]
    Channel(#[from] mpsc::error::SendError<Signal>),

    #[error("Invalid Config: {0}")]
    Config(#[from] ConfigError)
}

pub type Result<T> = std::result::Result<T, Error>;