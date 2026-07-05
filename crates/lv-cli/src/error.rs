use thiserror::Error;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("not logged in — run `lorevault login` first")]
    NotLoggedIn,

    #[error("server error ({status}): {message}")]
    Api { status: u16, message: String },

    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("config error: {0}")]
    Config(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, CliError>;
