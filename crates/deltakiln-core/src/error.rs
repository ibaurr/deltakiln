use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An I/O failure, tagged with the path it happened on.
    #[error("{}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// `deltakiln.toml` could not be parsed or holds an invalid value.
    #[error("invalid config {}: {message}", path.display())]
    Config { path: PathBuf, message: String },

    /// A problem with one source file (front matter, routing, ...).
    /// `path` is the source path relative to its content directory.
    #[error("{path}: {message}")]
    Content { path: String, message: String },

    /// Two inputs want to write the same output location.
    #[error("output conflict: {first} and {second} both produce `{output}`")]
    Collision {
        output: String,
        first: String,
        second: String,
    },

    /// A template failed to load or render.
    #[error("template error: {0}")]
    Template(String),

    /// Anything else that stops a build (bad arguments, unsafe output dir, ...).
    #[error("{0}")]
    Other(String),
}

impl Error {
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Error::Io {
            path: path.into(),
            source,
        }
    }

    pub fn content(path: impl Into<String>, message: impl Into<String>) -> Self {
        Error::Content {
            path: path.into(),
            message: message.into(),
        }
    }
}
