#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Creg {
    PC,
    SR,
    EPC,
    ESR,
    ECause,
    EData,
    EvBase,
}

impl std::fmt::Display for Creg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PC => write!(f, "%pc%"),
            Self::SR => write!(f, "%sr%"),
            Self::EPC => write!(f, "%epc%"),
            Self::ESR => write!(f, "%esr%"),
            Self::ECause => write!(f, "%ecause%"),
            Self::EData => write!(f, "%edata%"),
            Self::EvBase => write!(f, "%evbase%"),
        }
    }
}
