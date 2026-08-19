use viv32_vo::VoError;

#[derive(Debug)]
pub enum LinkerError {
    DuplicateLabel(String),
    UnknownLabel(String),
    InvalidRange(String),
    ObjectError(String),
    IOError(String),
}

impl From<VoError> for LinkerError {
    fn from(err: VoError) -> Self {
        Self::ObjectError(err.to_string())
    }
}

impl From<std::io::Error> for LinkerError {
    fn from(err: std::io::Error) -> Self {
        Self::IOError(err.to_string())
    }
}

impl std::fmt::Display for LinkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkerError::DuplicateLabel(message)
            | LinkerError::UnknownLabel(message)
            | LinkerError::InvalidRange(message)
            | LinkerError::IOError(message)
            | LinkerError::ObjectError(message) => {
                write!(f, "{}", message)
            }
        }
    }
}

impl std::error::Error for LinkerError {}
