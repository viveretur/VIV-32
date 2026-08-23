use viv32_isa::EncodeError;
use viv32_vo::VoError;

#[derive(Debug)]
pub struct AssemblerError {
    pub err: ParseError,
    pub filename: String,
    pub line: usize,
}

impl std::fmt::Display for AssemblerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}:{}] {}", self.filename, self.line, self.err)
    }
}

impl std::error::Error for AssemblerError {}

#[derive(Debug)]
pub enum ParseError {
    InvalidInstruction(String),
    InvalidName(String),
    DuplicateLabel(String),
    ObjectError(VoError),
    IOError(std::io::Error),
    DataAlignmentError(String),
    InvalidStringLiteral(String),
    InvalidNumber(String),
    EncodeError(EncodeError),
}

impl From<VoError> for ParseError {
    fn from(err: VoError) -> Self {
        Self::ObjectError(err)
    }
}

impl From<EncodeError> for ParseError {
    fn from(err: EncodeError) -> Self {
        Self::EncodeError(err)
    }
}

impl From<std::io::Error> for ParseError {
    fn from(err: std::io::Error) -> Self {
        Self::IOError(err)
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidInstruction(message) => write!(f, "{}", message),
            ParseError::InvalidName(name) => write!(f, "Invalid symbol name: {}", name),
            ParseError::DuplicateLabel(message) => write!(f, "{}", message),
            ParseError::ObjectError(err) => write!(f, "{}", err),
            ParseError::IOError(err) => write!(f, "{}", err),
            ParseError::DataAlignmentError(message) => write!(f, "{}", message),
            ParseError::InvalidNumber(message) => write!(f, "{}", message),
            ParseError::InvalidStringLiteral(message) => write!(f, "{}", message),
            ParseError::EncodeError(err) => write!(f, "{:?}", err),
        }
    }
}

impl std::error::Error for ParseError {}
