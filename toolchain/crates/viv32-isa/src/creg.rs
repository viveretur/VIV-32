#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Creg {
    PC = 0x0,
    SR = 0x1,
    EPC = 0x2,
    ESR = 0x3,
    ECause = 0x4,
    EData = 0x5,
    EvBase = 0x6,
}

impl std::fmt::Display for Creg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PC => write!(f, "%pc"),
            Self::SR => write!(f, "%sr"),
            Self::EPC => write!(f, "%epc"),
            Self::ESR => write!(f, "%esr"),
            Self::ECause => write!(f, "%ecause"),
            Self::EData => write!(f, "%edata"),
            Self::EvBase => write!(f, "%evbase"),
        }
    }
}
