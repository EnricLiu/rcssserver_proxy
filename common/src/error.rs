use thiserror;
use toml;


#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("Failed to parse config file, {0}")]
    Deserialize(#[from] toml::de::Error),

    #[error("Conflict Fields: {0:?}")]
    ConflictFields(Vec<&'static str>),

    #[error("Invalid Value on \"{0}\" due to {1}")]
    InvalidValue(&'static str, String),

    #[error("Missing Field: {0}")]
    MissingField(&'static str),
}