use crate::{bss::BssParseError, symbol::SymbolParseError};

#[derive(Debug)]
pub enum VoError {
    InvalidObject(String),
    InvalidRelocation(String),
    InvalidSign(String),
    IOError(std::io::Error),
    ShiftError,
    WidthError,
    BssError(BssParseError),
    SymbolError(SymbolParseError),
}

impl From<std::io::Error> for VoError {
    fn from(err: std::io::Error) -> Self {
        VoError::IOError(err)
    }
}

impl From<BssParseError> for VoError {
    fn from(err: BssParseError) -> Self {
        VoError::BssError(err)
    }
}

impl From<SymbolParseError> for VoError {
    fn from(err: SymbolParseError) -> Self {
        VoError::SymbolError(err)
    }
}

impl std::fmt::Display for VoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidObject(obj) => write!(f, "InvalidObject: {}", obj),
            Self::InvalidRelocation(rel) => write!(f, "InvalidRelocation: {}", rel),
            Self::InvalidSign(sign) => write!(f, "InvalidSign: {}", sign),
            Self::ShiftError => write!(f, "ShiftError"),
            Self::WidthError => write!(f, "WidthError"),
            Self::BssError(err) => write!(f, "BssError: {:?}", err),
            Self::SymbolError(err) => write!(f, "SymbolError: {:?}", err),
            Self::IOError(err) => write!(f, "IOError: {}", err),
        }
    }
}

impl std::error::Error for VoError {}
