#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ExceptionCause {
    Reset = 0x00,
    IllegalInstruction = 0x01,
    MisalignedInstructionFetch = 0x02,
    MisalignedDataAccess = 0x03,
    SoftwareTrap = 0x04,
    SystemCall = 0x05,
    TimerInterrupt = 0x06,
    ExternalInterrupt = 0x07,
    BusError = 0x08,
}

impl ExceptionCause {
    pub const SLOT_SIZE: u32 = 16;
    pub const TABLE_SIZE: u32 = 9 * Self::SLOT_SIZE;

    pub fn code(self) -> u32 {
        self as u32
    }

    pub fn vector_offset(self) -> u32 {
        self.code() * Self::SLOT_SIZE
    }
}

impl TryFrom<u32> for ExceptionCause {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Reset),
            1 => Ok(Self::IllegalInstruction),
            2 => Ok(Self::MisalignedInstructionFetch),
            3 => Ok(Self::MisalignedDataAccess),
            4 => Ok(Self::SoftwareTrap),
            5 => Ok(Self::SystemCall),
            6 => Ok(Self::TimerInterrupt),
            7 => Ok(Self::ExternalInterrupt),
            8 => Ok(Self::BusError),
            _ => Err(()),
        }
    }
}
