use std::{fmt, num::ParseIntError, str::FromStr};

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Symbol {
    pub offset: u32,
    pub name: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SymbolParseError {
    MissingOffset,
    MissingName,
    ExtraFields,
    InvalidOffset(ParseIntError),
}

impl TryFrom<&str> for Symbol {
    type Error = SymbolParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut parts = value.split_whitespace();

        let offset = parts.next().ok_or(SymbolParseError::MissingOffset)?;
        let name = parts.next().ok_or(SymbolParseError::MissingName)?;

        if parts.next().is_some() {
            return Err(SymbolParseError::ExtraFields);
        }

        let offset = u32::from_str_radix(offset, 16).map_err(SymbolParseError::InvalidOffset)?;

        Ok(Self {
            offset,
            name: name.to_string(),
        })
    }
}

impl FromStr for Symbol {
    type Err = SymbolParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:08X} {}", self.offset, self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_to_string_uses_vo_format() {
        let symbol = Symbol {
            offset: 0x100,
            name: "label0".to_string(),
        };

        assert_eq!(symbol.to_string(), "00000100 label0");
    }

    #[test]
    fn symbol_try_from_parses_vo_format() {
        let symbol = Symbol::try_from("00000100 label0").unwrap();

        assert_eq!(
            symbol,
            Symbol {
                offset: 0x100,
                name: "label0".to_string(),
            }
        );
    }

    #[test]
    fn symbol_try_from_allows_lowercase_hex() {
        let symbol = Symbol::try_from("00000100 label0").unwrap();
        assert_eq!(symbol.offset, 0x100);
    }

    #[test]
    fn symbol_try_from_rejects_missing_offset() {
        assert_eq!(Symbol::try_from(""), Err(SymbolParseError::MissingOffset));
    }

    #[test]
    fn symbol_try_from_rejects_missing_name() {
        assert_eq!(
            Symbol::try_from("00000100"),
            Err(SymbolParseError::MissingName)
        );
    }

    #[test]
    fn symbol_try_from_rejects_extra_fields() {
        assert_eq!(
            Symbol::try_from("00000100 label0 extra"),
            Err(SymbolParseError::ExtraFields)
        );
    }
}
