use std::fmt;
use std::io;
use std::path::PathBuf;

/// Library-level error. The CLI maps these to exit codes and messages.
#[derive(Debug)]
pub enum Error {
    PresetNotFound {
        name: String,
        searched: Vec<String>,
    },
    PresetParse {
        source: String,
        message: String,
    },
    InvalidParam {
        message: String,
    },
    Wav(hound::Error),
    Io {
        path: Option<PathBuf>,
        source: io::Error,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::PresetNotFound { name, searched } => {
                write!(
                    f,
                    "preset `{name}` not found (looked in: {})",
                    searched.join(", ")
                )
            }
            Error::PresetParse { source, message } => {
                write!(f, "failed to parse preset {source}: {message}")
            }
            Error::InvalidParam { message } => write!(f, "{message}"),
            Error::Wav(err) => write!(f, "wav error: {err}"),
            Error::Io { path, source } => match path {
                Some(p) => write!(f, "io error ({}): {source}", p.display()),
                None => write!(f, "io error: {source}"),
            },
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Wav(err) => Some(err),
            Error::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<hound::Error> for Error {
    fn from(value: hound::Error) -> Self {
        Error::Wav(value)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
