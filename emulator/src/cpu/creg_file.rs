use crate::lifecycle::Reset;

use super::{ExceptionCause, ProgramCounter, StatusRegister};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Creg {
    PC,
    SR,
    EPC,
    ECause,
    EAddr,
    EvBase,
}

#[derive(Debug)]
pub struct CregFile {
    pc: ProgramCounter,
    sr: StatusRegister,
    epc: u32,
    ecause: u32,
    eaddr: u32,
    evbase: u32,
}

impl CregFile {
    pub fn new() -> Self {
        Self {
            pc: ProgramCounter::new(),
            sr: StatusRegister::new(),
            epc: 0,
            ecause: ExceptionCause::Reset as u32,
            eaddr: 0,
            evbase: 0,
        }
    }

    pub fn pc(&self) -> u32 {
        self.pc.get()
    }

    pub fn sr(&self) -> u32 {
        self.sr.get()
    }

    pub fn epc(&self) -> u32 {
        self.epc
    }

    pub fn eaddr(&self) -> u32 {
        self.eaddr
    }

    pub fn evbase(&self) -> u32 {
        self.evbase
    }

    pub(crate) fn reset(&mut self) {
        self.pc
            .set(self.evbase + ExceptionCause::Reset.vector_offset());
        self.sr.reset();
        self.epc = 0;
        self.ecause = ExceptionCause::Reset as u32;
        self.evbase = 0;
    }

    pub(crate) fn advance_pc_word(&mut self) {
        self.pc.advance_word();
    }

    pub(crate) fn set_pc(&mut self, value: u32) {
        self.pc.set(value);
    }

    pub(crate) fn set_sr(&mut self, value: u32) {
        self.sr.set(value);
    }

    pub(crate) fn ei(&mut self) {
        self.sr.set_interrupt_enable(true);
    }

    pub(crate) fn di(&mut self) {
        self.sr.set_interrupt_enable(false);
    }

    pub(crate) fn raise_exception(&mut self, cause: ExceptionCause, fault_addr: u32) {
        // EPC is the continuation PC, not necessarily the faulting instruction PC.
        // The fetch/decode/execute pipeline advances PC before execute, so exception
        // entry records the address that IRET should resume at.
        self.sr.set_interrupt_enable(false);
        self.epc = self.pc.get();
        self.ecause = cause as u32;
        self.eaddr = fault_addr;
        self.pc.set(self.evbase + cause.vector_offset());
    }

    pub(crate) fn iret(&mut self) {
        self.pc.set(self.epc);
        self.sr.set_interrupt_enable(true);
    }

    pub(crate) fn set_eaddr(&mut self, value: u32) {
        self.eaddr = value;
    }

    pub(crate) fn read_register(&self, reg: Creg) -> u32 {
        match reg {
            Creg::PC => self.pc.get(),
            Creg::SR => self.sr.get(),
            Creg::EPC => self.epc,
            Creg::ECause => self.ecause,
            Creg::EAddr => self.eaddr,
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
            Creg::ECause => {
                self.ecause = value;
            }
            Creg::EAddr => {
                self.eaddr = value;
            }
            Creg::EvBase => {
                self.evbase = value;
            }
        }
    }
}
