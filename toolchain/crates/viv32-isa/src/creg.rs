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

