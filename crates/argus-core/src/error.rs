use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArgusError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Graph database error: {0}")]
    Graph(String),

    #[error("Extraction error: {0}")]
    Extraction(String),

    #[error("Reasoning error: {0}")]
    Reasoning(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Agent error ({agent}): {message}")]
    Agent { agent: String, message: String },

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type Result<T> = std::result::Result<T, ArgusError>;
