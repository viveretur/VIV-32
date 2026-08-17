use std::{fmt, num::ParseIntError, str::FromStr};

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct Bss {
    pub alignment: u32,
    pub size: u32,
    pub name: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum BssParseError {
    MissingAlignment,
    MissingSize,
    MissingName,
    InvalidAlignment(ParseIntError),
    InvalidSize(ParseIntError),
    ExtraFields,
}

impl TryFrom<&str> for Bss {
    type Error = BssParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut parts = value.split_whitespace();

        let alignment = parts.next().ok_or(BssParseError::MissingAlignment)?;
        let size = parts.next().ok_or(BssParseError::MissingSize)?;
        let name = parts.next().ok_or(BssParseError::MissingName)?;

        if parts.next().is_some() {
            return Err(BssParseError::ExtraFields);
        }

        let alignment =
            u32::from_str_radix(alignment, 16).map_err(BssParseError::InvalidAlignment)?;
        let size = u32::from_str_radix(size, 16).map_err(BssParseError::InvalidSize)?;

        Ok(Self {
            alignment,
            size,
            name: name.to_string(),
        })
    }
}

impl FromStr for Bss {
    type Err = BssParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl fmt::Display for Bss {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:08X} {:08X} {}", self.alignment, self.size, self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bss_to_string_uses_vo_format() {
        let bss = Bss {
            alignment: 0x4,
            size: 0x100,
            name: "label0".to_string(),
        };

        assert_eq!(bss.to_string(), "00000004 00000100 label0");
    }

    #[test]
    fn bss_try_from_parses_vo_format() {
        let bss = Bss::try_from("00000004 00000100 label0").unwrap();

        assert_eq!(
            bss,
            Bss {
                alignment: 0x4,
                size: 0x100,
                name: "label0".to_string(),
            }
        );
    }

    #[test]
    fn bss_try_from_allows_lowercase_hex() {
        let bss = Bss::try_from("00000004 00000a00 label0").unwrap();
        assert_eq!(bss.size, 0xA00);
    }

    #[test]
    fn bss_try_from_rejects_missing_alignment() {
        assert_eq!(Bss::try_from(""), Err(BssParseError::MissingAlignment));
    }

    #[test]
    fn bss_try_from_rejects_missing_name() {
        assert_eq!(
            Bss::try_from("00000004 00000100"),
            Err(BssParseError::MissingName)
        );
    }

    #[test]
    fn bss_try_from_rejects_extra_fields() {
        assert_eq!(
            Bss::try_from("00000004 00000100 label0 extra"),
            Err(BssParseError::ExtraFields)
        );
    }
}
