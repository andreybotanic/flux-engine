use std::{error::Error, fmt};

/// Public error returned by SDK APIs and plugin handlers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PluginError {
    /// One plugin callback received malformed data.
    InvalidArgument(String),
    /// The requested engine API is not available in the current dispatch scope.
    ApiUnavailable(&'static str),
    /// The engine does not implement the requested operation yet.
    Unsupported(&'static str),
    /// Generic plugin-side failure with a readable message.
    Message(String),
}

impl PluginError {
    /// Builds a generic plugin error from owned text.
    pub fn message(message: impl Into<String>) -> Self {
        Self::Message(message.into())
    }
}

impl From<String> for PluginError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidArgument(message) => write!(f, "invalid argument: {message}"),
            Self::ApiUnavailable(api) => write!(f, "API '{api}' is unavailable outside dispatch"),
            Self::Unsupported(api) => write!(f, "API '{api}' is not supported by the host"),
            Self::Message(message) => f.write_str(message),
        }
    }
}

impl Error for PluginError {}
