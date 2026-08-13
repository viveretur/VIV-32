use crate::Lifecycle;

use super::{ExceptionCause, ProgramCounter, StatusRegister};

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

#[derive(Debug)]
pub struct CregFile {
    pc: ProgramCounter,
    sr: StatusRegister,
    epc: u32,
    esr: StatusRegister,
    ecause: u32,
    edata: u32,
    evbase: u32,
}

impl CregFile {
    pub fn new() -> Self {
        Self {
            pc: ProgramCounter::new(),
            sr: StatusRegister::new(),
            epc: 0,
            esr: StatusRegister::new(),
            ecause: ExceptionCause::Reset as u32,
            edata: 0,
            evbase: 0,
        }
    }

    pub(crate) fn sr(&self) -> &StatusRegister {
        &self.sr
    }

    pub(crate) fn sr_mut(&mut self) -> &mut StatusRegister {
        &mut self.sr
    }

    pub(crate) fn update_sr_flags(
        &mut self,
        arithmetic_error: bool,
        zero: bool,
        negative: bool,
        carry: bool,
        overflow: bool,
    ) {
        self.sr.set_arithmetic_error(arithmetic_error);
        self.sr.set_zero(zero);
        self.sr.set_negative(negative);
        self.sr.set_carry(carry);
        self.sr.set_overflow(overflow);
    }

    pub(crate) fn advance_pc_word(&mut self) {
        self.pc.advance_word();
    }

    pub(crate) fn ei(&mut self) {
        self.sr.set_interrupt_enable(true);
    }

    pub(crate) fn di(&mut self) {
        self.sr.set_interrupt_enable(false);
    }

    pub(crate) fn raise_exception(&mut self, cause: ExceptionCause, data: u32) {
        // EPC is the continuation PC, not necessarily the faulting instruction PC.
        // The fetch/decode/execute pipeline advances PC before execute, so exception
        // entry records the address that IRET should resume at.
        self.esr.set(self.sr.get());
        self.sr.set_interrupt_enable(false);
        self.epc = self.pc.get();
        self.ecause = cause as u32;
        self.edata = data;
        self.pc.set(self.evbase + cause.vector_offset());
    }

    pub(crate) fn iret(&mut self) {
        self.pc.set(self.epc);
        self.sr.set(self.esr.get());
        // IE gets restored with the write to SR from ESR.
    }

    pub(crate) fn read_register(&self, reg: Creg) -> u32 {
        match reg {
            Creg::PC => self.pc.get(),
            Creg::SR => self.sr.get(),
            Creg::EPC => self.epc,
            Creg::ESR => self.esr.get(),
            Creg::ECause => self.ecause,
            Creg::EData => self.edata,
            Creg::EvBase => self.evbase,
        }
    }

    pub(crate) fn write_register(&mut self, reg: Creg, value: u32) {
        match reg {
            Creg::PC => {
                self.pc.set(value);
            }
            Creg::SR => {
                self.sr.set(value);
            }
            Creg::EPC => {
                self.epc = value;
            }
            Creg::ESR => {
                self.esr.set(value);
            }
            Creg::ECause => {
                self.ecause = value;
            }
            Creg::EData => {
                self.edata = value;
            }
            Creg::EvBase => {
                self.evbase = value;
            }
        }
    }
}

impl Lifecycle for CregFile {
    fn init(&mut self) {
        self.pc
            .set(self.evbase + ExceptionCause::Reset.vector_offset());
        self.sr.reset();
        self.epc = 0;
        self.ecause = ExceptionCause::Reset as u32;
        self.evbase = 0;
    }

    fn reset(&mut self) {
        self.init();
    }
}
