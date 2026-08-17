use crate::VoError;
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RelocationBase {
    Absolute,
    Relative,
}

impl TryFrom<&str> for RelocationBase {
    type Error = VoError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "abs" => Ok(RelocationBase::Absolute),
            "rel" => Ok(RelocationBase::Relative),
            _ => Err(VoError::InvalidRelocation(value.to_owned())),
        }
    }
}

impl FromStr for RelocationBase {
    type Err = VoError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl fmt::Display for RelocationBase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Absolute => write!(f, "abs"),
            Self::Relative => write!(f, "rel"),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum RelocationSign {
    Unsigned,
    Signed,
}

impl TryFrom<&str> for RelocationSign {
    type Error = VoError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "u" => Ok(RelocationSign::Unsigned),
            "s" => Ok(RelocationSign::Signed),
            _ => Err(VoError::InvalidSign(value.to_owned())),
        }
    }
}

impl FromStr for RelocationSign {
    type Err = VoError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl fmt::Display for RelocationSign {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsigned => write!(f, "u"),
            Self::Signed => write!(f, "s"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Relocation {
    pub patch_offset: u32,
    pub symbol: String,
    pub addend: i32,
    pub base: RelocationBase,
    pub sign: RelocationSign,
    pub value_shift: u8,
    pub width: u8,
    pub field_shift: u8,
}

impl Relocation {
    pub fn new(
        patch_offset: u32,
        symbol: String,
        addend: i32,
        base: RelocationBase,
        sign: RelocationSign,
        value_shift: u8,
        width: u8,
        field_shift: u8,
    ) -> Self {
        Self {
            patch_offset,
            symbol,
            addend,
            base,
            sign,
            value_shift,
            width,
            field_shift,
        }
    }
}

impl TryFrom<&str> for Relocation {
    type Error = VoError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let fields: Vec<&str> = value.split_whitespace().collect();

        if fields.len() != 8 {
            return Err(VoError::InvalidRelocation(format!(
                "expected 8 fields, found {}: {value}",
                fields.len()
            )));
        }

        Ok(Self {
            patch_offset: parse_hex_u32(fields[0], "patch_offset", value)?,
            symbol: fields[1].to_owned(),
            addend: parse_i32(fields[2], "addend", value)?,
            base: RelocationBase::try_from(fields[3])?,
            sign: RelocationSign::try_from(fields[4])?,
            value_shift: parse_u8(fields[5], "value_shift", value)?,
            width: parse_u8(fields[6], "width", value)?,
            field_shift: parse_u8(fields[7], "field_shift", value)?,
        })
    }
}

impl FromStr for Relocation {
    type Err = VoError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}

impl fmt::Display for Relocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:08X} {} {} {} {} {} {} {}",
            self.patch_offset,
            self.symbol,
            self.addend,
            self.base,
            self.sign,
            self.value_shift,
            self.width,
            self.field_shift
        )
    }
}

fn parse_hex_u32(field: &str, name: &str, row: &str) -> Result<u32, VoError> {
    u32::from_str_radix(field, 16).map_err(|err| {
        VoError::InvalidRelocation(format!(
            "invalid hex `{name}` value `{field}` in relocation row `{row}`: {err}"
        ))
    })
}

fn parse_i32(field: &str, name: &str, row: &str) -> Result<i32, VoError> {
    field.parse::<i32>().map_err(|err| {
        VoError::InvalidRelocation(format!(
            "invalid `{name}` value `{field}` in relocation row `{row}`: {err}"
        ))
    })
}

fn parse_u8(field: &str, name: &str, row: &str) -> Result<u8, VoError> {
    field.parse::<u8>().map_err(|err| {
        VoError::InvalidRelocation(format!(
            "invalid `{name}` value `{field}` in relocation row `{row}`: {err}"
        ))
    })
}

#[test]
fn relocation_round_trip() {
    let row = "00000080 _start -4 rel s 2 26 0";

    let relocation = Relocation::try_from(row).unwrap();

    assert_eq!(relocation.patch_offset, 0x80);
    assert_eq!(relocation.symbol, "_start");
    assert_eq!(relocation.addend, -4);
    assert_eq!(relocation.base, RelocationBase::Relative);
    assert_eq!(relocation.sign, RelocationSign::Signed);
    assert_eq!(relocation.value_shift, 2);
    assert_eq!(relocation.width, 26);
    assert_eq!(relocation.field_shift, 0);

    assert_eq!(relocation.to_string(), row);
}
