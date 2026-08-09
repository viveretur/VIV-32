// This file is generated from spec/isa/viv32-isa.yaml.
// Do not edit by hand.
//
// Source: spec/isa/viv32-isa.yaml

#![allow(dead_code)]

pub mod architecture {
    pub const NAME: &str = "VIV-32";
    pub const VERSION: &str = "0.1-draft";
    pub const PREFIX: &str = "viv32";
    pub const ENDIAN: &str = "big";
    pub const WORD_BITS: u32 = 0x20;
    pub const ADDRESS_BITS: u32 = 0x20;
    pub const INSTRUCTION_BITS: u32 = 0x20;
    pub const OPCODE_SHIFT: u32 = 0x1A;
    pub const OPCODE_WIDTH: u32 = 0x6;
    pub const OPCODE_MASK: u32 = 0x3F;
}

pub mod gpr {
    pub const WIDTH_BITS: u32 = 0x20;
    pub const COUNT: u32 = 0x10;
    pub const ENCODING_BITS: u32 = 0x4;

    pub const R0: usize = 0x0;
    pub const R1: usize = 0x1;
    pub const R2: usize = 0x2;
    pub const R3: usize = 0x3;
    pub const R4: usize = 0x4;
    pub const R5: usize = 0x5;
    pub const R6: usize = 0x6;
    pub const R7: usize = 0x7;
    pub const R8: usize = 0x8;
    pub const R9: usize = 0x9;
    pub const R10: usize = 0xA;
    pub const R11: usize = 0xB;
    pub const R12: usize = 0xC;
    pub const R13: usize = 0xD;
    pub const R14: usize = 0xE;
    pub const R15: usize = 0xF;

    /// Alias for r0
    pub const ZERO: usize = 0x0;
    /// Alias for r14
    pub const SP: usize = 0xE;
    /// Alias for r15
    pub const LR: usize = 0xF;
    /// Alias for r13
    pub const FP: usize = 0xD;
}

pub mod creg {
    /// Program counter
    pub const PC: usize = 0x0;
    /// Status register
    pub const SR: usize = 0x1;
    /// Exception program counter
    pub const EPC: usize = 0x2;
    /// Exception cause
    pub const ECAUSE: usize = 0x3;
    /// Exception address
    pub const EADDR: usize = 0x4;
    /// Exception vector base address
    pub const EVBASE: usize = 0x5;
}

pub mod sr {
    /// Negative flag
    pub const N_BIT: u32 = 0x0;
    pub const N_MASK: u32 = 1u32 << N_BIT;

    /// Zero flag
    pub const Z_BIT: u32 = 0x1;
    pub const Z_MASK: u32 = 1u32 << Z_BIT;

    /// Carry flag
    pub const C_BIT: u32 = 0x2;
    pub const C_MASK: u32 = 1u32 << C_BIT;

    /// Signed overflow flag
    pub const V_BIT: u32 = 0x3;
    pub const V_MASK: u32 = 1u32 << V_BIT;

    /// Arithmetic error flag
    pub const E_BIT: u32 = 0x4;
    pub const E_MASK: u32 = 1u32 << E_BIT;

    /// Interrupt enable bit
    pub const IE_BIT: u32 = 0x5;
    pub const IE_MASK: u32 = 1u32 << IE_BIT;

}

pub mod format {
    pub mod r {
        pub const FUNC_SHIFT: u32 = 0xC;
        pub const FUNC_WIDTH: u32 = 0xE;
        pub const FUNC_MASK: u32 = 0x03FFF000;

        pub const RD_SHIFT: u32 = 0x8;
        pub const RD_WIDTH: u32 = 0x4;
        pub const RD_MASK: u32 = 0x00000F00;

        pub const RA_SHIFT: u32 = 0x4;
        pub const RA_WIDTH: u32 = 0x4;
        pub const RA_MASK: u32 = 0x000000F0;

        pub const RB_SHIFT: u32 = 0x0;
        pub const RB_WIDTH: u32 = 0x4;
        pub const RB_MASK: u32 = 0x0000000F;

    }

    pub mod r2 {
        pub const FUNC_SHIFT: u32 = 0x10;
        pub const FUNC_WIDTH: u32 = 0xA;
        pub const FUNC_MASK: u32 = 0x03FF0000;

        pub const RD0_SHIFT: u32 = 0xC;
        pub const RD0_WIDTH: u32 = 0x4;
        pub const RD0_MASK: u32 = 0x0000F000;

        pub const RD1_SHIFT: u32 = 0x8;
        pub const RD1_WIDTH: u32 = 0x4;
        pub const RD1_MASK: u32 = 0x00000F00;

        pub const RA_SHIFT: u32 = 0x4;
        pub const RA_WIDTH: u32 = 0x4;
        pub const RA_MASK: u32 = 0x000000F0;

        pub const RB_SHIFT: u32 = 0x0;
        pub const RB_WIDTH: u32 = 0x4;
        pub const RB_MASK: u32 = 0x0000000F;

    }

    pub mod i {
        pub const MODE_SHIFT: u32 = 0x18;
        pub const MODE_WIDTH: u32 = 0x2;
        pub const MODE_MASK: u32 = 0x03000000;

        pub const IMM16_SHIFT: u32 = 0x8;
        pub const IMM16_WIDTH: u32 = 0x10;
        pub const IMM16_MASK: u32 = 0x00FFFF00;

        pub const RD_SHIFT: u32 = 0x4;
        pub const RD_WIDTH: u32 = 0x4;
        pub const RD_MASK: u32 = 0x000000F0;

        pub const RA_SHIFT: u32 = 0x0;
        pub const RA_WIDTH: u32 = 0x4;
        pub const RA_MASK: u32 = 0x0000000F;

    }

    pub mod u {
        pub const MODE_SHIFT: u32 = 0x14;
        pub const MODE_WIDTH: u32 = 0x6;
        pub const MODE_MASK: u32 = 0x03F00000;

        pub const IMM16_SHIFT: u32 = 0x4;
        pub const IMM16_WIDTH: u32 = 0x10;
        pub const IMM16_MASK: u32 = 0x000FFFF0;

        pub const RD_SHIFT: u32 = 0x0;
        pub const RD_WIDTH: u32 = 0x4;
        pub const RD_MASK: u32 = 0x0000000F;

    }

    pub mod m {
        pub const SX_SHIFT: u32 = 0x19;
        pub const SX_WIDTH: u32 = 0x1;
        pub const SX_MASK: u32 = 0x02000000;

        pub const SIZE_SHIFT: u32 = 0x17;
        pub const SIZE_WIDTH: u32 = 0x2;
        pub const SIZE_MASK: u32 = 0x01800000;

        pub const OFFSET15_SHIFT: u32 = 0x8;
        pub const OFFSET15_WIDTH: u32 = 0xF;
        pub const OFFSET15_MASK: u32 = 0x007FFF00;

        pub const RD_RS_SHIFT: u32 = 0x4;
        pub const RD_RS_WIDTH: u32 = 0x4;
        pub const RD_RS_MASK: u32 = 0x000000F0;

        pub const BASE_SHIFT: u32 = 0x0;
        pub const BASE_WIDTH: u32 = 0x4;
        pub const BASE_MASK: u32 = 0x0000000F;

    }

    pub mod bf {
        pub const COND_SHIFT: u32 = 0x16;
        pub const COND_WIDTH: u32 = 0x4;
        pub const COND_MASK: u32 = 0x03C00000;

        pub const OFFSET22_SHIFT: u32 = 0x0;
        pub const OFFSET22_WIDTH: u32 = 0x16;
        pub const OFFSET22_MASK: u32 = 0x003FFFFF;

    }

    pub mod bc {
        pub const COND_SHIFT: u32 = 0x16;
        pub const COND_WIDTH: u32 = 0x4;
        pub const COND_MASK: u32 = 0x03C00000;

        pub const OFFSET14_SHIFT: u32 = 0x8;
        pub const OFFSET14_WIDTH: u32 = 0xE;
        pub const OFFSET14_MASK: u32 = 0x003FFF00;

        pub const RA_SHIFT: u32 = 0x4;
        pub const RA_WIDTH: u32 = 0x4;
        pub const RA_MASK: u32 = 0x000000F0;

        pub const RB_SHIFT: u32 = 0x0;
        pub const RB_WIDTH: u32 = 0x4;
        pub const RB_MASK: u32 = 0x0000000F;

    }

    pub mod j {
        pub const OFFSET26_SHIFT: u32 = 0x0;
        pub const OFFSET26_WIDTH: u32 = 0x1A;
        pub const OFFSET26_MASK: u32 = 0x03FFFFFF;

    }

    pub mod jr {
        pub const FUNC_SHIFT: u32 = 0x8;
        pub const FUNC_WIDTH: u32 = 0x12;
        pub const FUNC_MASK: u32 = 0x03FFFF00;

        pub const RD_SHIFT: u32 = 0x4;
        pub const RD_WIDTH: u32 = 0x4;
        pub const RD_MASK: u32 = 0x000000F0;

        pub const TARGET_SHIFT: u32 = 0x0;
        pub const TARGET_WIDTH: u32 = 0x4;
        pub const TARGET_MASK: u32 = 0x0000000F;

    }

    pub mod x {
        pub const SYSFUNC_SHIFT: u32 = 0x8;
        pub const SYSFUNC_WIDTH: u32 = 0x12;
        pub const SYSFUNC_MASK: u32 = 0x03FFFF00;

        pub const RD_SHIFT: u32 = 0x4;
        pub const RD_WIDTH: u32 = 0x4;
        pub const RD_MASK: u32 = 0x000000F0;

        pub const RS_SHIFT: u32 = 0x0;
        pub const RS_WIDTH: u32 = 0x4;
        pub const RS_MASK: u32 = 0x0000000F;

        pub const SYSOP_SHIFT: u32 = 0x14;
        pub const SYSOP_WIDTH: u32 = 0x6;
        pub const SYSOP_MASK: u32 = 0x03F00000;

        pub const PAYLOAD_SHIFT: u32 = 0x8;
        pub const PAYLOAD_WIDTH: u32 = 0xC;
        pub const PAYLOAD_MASK: u32 = 0x000FFF00;

    }

}

pub mod opcode {
    /// System/control
    pub const SYSTEM_CONTROL: u32 = 0x0;
    /// Register ALU
    pub const REGISTER_ALU: u32 = 0x1;
    /// Immediate arithmetic/compare
    pub const IMMEDIATE_ARITHMETIC_COMPARE: u32 = 0x2;
    /// Immediate logical
    pub const IMMEDIATE_LOGICAL: u32 = 0x3;
    /// Register shifts
    pub const REGISTER_SHIFT: u32 = 0x4;
    /// Immediate shifts
    pub const IMMEDIATE_SHIFT: u32 = 0x5;
    /// Bit immediate
    pub const BIT_IMMEDIATE: u32 = 0x6;
    /// Multiply/divide
    pub const MULTIPLY_DIVIDE: u32 = 0x7;
    /// Constant construction
    pub const CONSTANT_CONSTRUCTION: u32 = 0x8;
    /// Load
    pub const LOAD: u32 = 0x9;
    /// Store
    pub const STORE: u32 = 0xA;
    /// Flag branch
    pub const FLAG_BRANCH: u32 = 0xB;
    /// Register branch
    pub const REGISTER_BRANCH: u32 = 0xC;
    /// PC-relative jump
    pub const PC_RELATIVE_JUMP: u32 = 0xD;
    /// PC-relative call
    pub const PC_RELATIVE_CALL: u32 = 0xE;
    /// Register jump/call
    pub const REGISTER_JUMP_CALL: u32 = 0xF;
}

pub mod func {
    pub mod register_alu {
        pub const ADD: u32 = 0x0;
        pub const SUB: u32 = 0x1;
        pub const AND: u32 = 0x2;
        pub const OR: u32 = 0x3;
        pub const XOR: u32 = 0x4;
        pub const NOT: u32 = 0x5;
        pub const NEG: u32 = 0x6;
        pub const CMP: u32 = 0x7;
    }

    pub mod register_shift {
        pub const SHL: u32 = 0x0;
        pub const SHR: u32 = 0x1;
        pub const SAR: u32 = 0x2;
    }

    pub mod multiply_divide {
        pub const MUL: u32 = 0x0;
        pub const MULU: u32 = 0x1;
        pub const DIV: u32 = 0x2;
        pub const DIVU: u32 = 0x3;
    }

    pub mod register_jump_call {
        pub const JR: u32 = 0x0;
        pub const JALR: u32 = 0x1;
    }

}

pub mod mode {
    pub mod immediate_arithmetic_compare {
        pub const ADDI: u32 = 0x0;
        pub const SUBI: u32 = 0x1;
        pub const CMPI: u32 = 0x2;
    }

    pub mod immediate_logical {
        pub const ANDI: u32 = 0x0;
        pub const ORI: u32 = 0x1;
        pub const XORI: u32 = 0x2;
    }

    pub mod immediate_shift {
        pub const SHLI: u32 = 0x0;
        pub const SHRI: u32 = 0x1;
        pub const SARI: u32 = 0x2;
    }

    pub mod bit_immediate {
        pub const BTST: u32 = 0x0;
        pub const BSET: u32 = 0x1;
        pub const BCLR: u32 = 0x2;
        pub const BTGL: u32 = 0x3;
    }

    pub mod constant_construction {
        pub const LUI: u32 = 0x0;
        pub const LLI: u32 = 0x1;
        pub const LHI: u32 = 0x2;
    }

}

pub mod condition {
    /// equal
    pub const EQ: u32 = 0x0;
    /// not equal
    pub const NE: u32 = 0x1;
    /// signed less than
    pub const LT: u32 = 0x2;
    /// signed less than or equal
    pub const LE: u32 = 0x3;
    /// signed greater than
    pub const GT: u32 = 0x4;
    /// signed greater than or equal
    pub const GE: u32 = 0x5;
    /// unsigned less than
    pub const LTU: u32 = 0x6;
    /// unsigned less than or equal
    pub const LEU: u32 = 0x7;
    /// unsigned greater than
    pub const GTU: u32 = 0x8;
    /// unsigned greater than or equal
    pub const GEU: u32 = 0x9;
    /// carry set
    pub const CS: u32 = 0xA;
    /// carry clear
    pub const CC: u32 = 0xB;
    /// overflow set
    pub const VS: u32 = 0xC;
    /// overflow clear
    pub const VC: u32 = 0xD;
    /// arithmetic error set
    pub const ES: u32 = 0xE;
    /// arithmetic error clear
    pub const EC: u32 = 0xF;
}

pub mod memory_size {
    pub const BYTE: u32 = 0x0;
    pub const BYTE_BITS: u32 = 0x8;
    pub const BYTE_ALIGNMENT: u32 = 0x1;

    pub const HALFWORD: u32 = 0x1;
    pub const HALFWORD_BITS: u32 = 0x10;
    pub const HALFWORD_ALIGNMENT: u32 = 0x2;

    pub const WORD: u32 = 0x2;
    pub const WORD_BITS: u32 = 0x20;
    pub const WORD_ALIGNMENT: u32 = 0x4;

    pub const RESERVED: u32 = 0x3;

}

pub mod sysop {
    pub const NOP: u32 = 0x0;
    pub const HALT: u32 = 0x1;
    pub const TRAP: u32 = 0x2;
    pub const SYSCALL: u32 = 0x3;
    pub const IRET: u32 = 0x4;
    pub const EI: u32 = 0x5;
    pub const DI: u32 = 0x6;
    pub const RDPC: u32 = 0x7;
    pub const MRS: u32 = 0x8;
    pub const MSR: u32 = 0x9;
}

pub mod exception_cause {
    /// Reset entry
    pub const RESET: u32 = 0x0;
    /// Undefined or reserved instruction encoding
    pub const ILLEGAL_INSTRUCTION: u32 = 0x1;
    /// Instruction fetch from a non-4-byte-aligned address
    pub const MISALIGNED_INSTRUCTION_FETCH: u32 = 0x2;
    /// Misaligned halfword or word load/store
    pub const MISALIGNED_DATA_ACCESS: u32 = 0x3;
    /// trap instruction
    pub const SOFTWARE_TRAP: u32 = 0x4;
    /// syscall instruction
    pub const SYSTEM_CALL: u32 = 0x5;
    /// Platform timer interrupt
    pub const TIMER_INTERRUPT: u32 = 0x6;
    /// Platform external interrupt
    pub const EXTERNAL_INTERRUPT: u32 = 0x7;
}

