use viv32_isa::{Creg, Instruction, decode, encode};

#[rustfmt::skip]
fn known_good_instructions() -> Vec<Instruction> {
    vec![
        // Control / X-type
        Instruction::Nop,
        Instruction::Halt,
        Instruction::SoftwareTrap { imm: 0x123 },
        Instruction::SystemCall,
        Instruction::IRet,
        Instruction::EI,
        Instruction::DI,
        Instruction::RdPc { rd: 1 },
        Instruction::Mrs { creg4: Creg::PC, rd: 2 },
        Instruction::Msr { creg4: Creg::PC, rs: 3 },

        // Register ALU / R-type
        Instruction::Add { rd: 1, ra: 2, rb: 3 },
        Instruction::Sub { rd: 1, ra: 2, rb: 3 },
        Instruction::And { rd: 1, ra: 2, rb: 3 },
        Instruction::Or  { rd: 1, ra: 2, rb: 3 },
        Instruction::Xor { rd: 1, ra: 2, rb: 3 },
        Instruction::Not { rd: 1, ra: 2 },
        Instruction::Neg { rd: 1, ra: 2 },
        Instruction::Cmp { ra: 2, rb: 3 },

        // Immediate arithmetic / compare
        Instruction::Addi { rd: 1, ra: 2, imm: 123 },
        Instruction::Subi { rd: 1, ra: 2, imm: 0xFFF_000C },
        Instruction::Cmpi { ra: 2, imm: u32::MAX },

        // Immediate logical
        Instruction::Andi { rd: 1, ra: 2, imm: 0x00FF },
        Instruction::Ori  { rd: 1, ra: 2, imm: 0x0F0F },
        Instruction::Xori { rd: 1, ra: 2, imm: 0xFFFF },

        // Register shifts
        Instruction::Shl { rd: 1, ra: 2, rb: 3 },
        Instruction::Shr { rd: 1, ra: 2, rb: 3 },
        Instruction::Sar { rd: 1, ra: 2, rb: 3 },

        // Immediate shifts
        Instruction::Shli { rd: 1, ra: 2, imm: 4 },
        Instruction::Shri { rd: 1, ra: 2, imm: 8 },
        Instruction::Sari { rd: 1, ra: 2, imm: 12 },

        // Bit immediate
        Instruction::Btst { ra: 2, imm: 7 },
        Instruction::Bset { rd: 1, ra: 2, imm: 7 },
        Instruction::Bclr { rd: 1, ra: 2, imm: 7 },
        Instruction::Btgl { rd: 1, ra: 2, imm: 7 },

        // Multiply / divide
        Instruction::Mul  { rd0: 1, rd1: 2, ra: 3, rb: 4 },
        Instruction::Mulu { rd0: 1, rd1: 2, ra: 3, rb: 4 },
        Instruction::Div  { rd0: 1, rd1: 2, ra: 3, rb: 4 },
        Instruction::Divu { rd0: 1, rd1: 2, ra: 3, rb: 4 },

        // Constant construction
        Instruction::Lui { rd: 1, imm16: 0x1234 },
        Instruction::Lli { rd: 1, imm16: 0x5678 },
        Instruction::Lhi { rd: 1, imm16: 0x9ABC },

        // Load / store
        Instruction::Lb  { rd: 1, base: 2, offset: -4 },
        Instruction::Lbu { rd: 1, base: 2, offset: 4 },
        Instruction::Lh  { rd: 1, base: 2, offset: -8 },
        Instruction::Lhu { rd: 1, base: 2, offset: 8 },
        Instruction::Lw  { rd: 1, base: 2, offset: 12 },
        Instruction::Sb  { rs: 1, base: 2, offset: 0 },
        Instruction::Sh  { rs: 1, base: 2, offset: 2 },
        Instruction::Sw  { rs: 1, base: 2, offset: 4 },

        // Flag branch / BF-type
        Instruction::BfEq  { offset: -4 },
        Instruction::BfNe  { offset: 4 },
        Instruction::BfLt  { offset: 8 },
        Instruction::BfLe  { offset: 12 },
        Instruction::BfGt  { offset: 16 },
        Instruction::BfGe  { offset: 20 },
        Instruction::BfLtu { offset: 24 },
        Instruction::BfLeu { offset: 28 },
        Instruction::BfGtu { offset: 32 },
        Instruction::BfGeu { offset: 36 },
        Instruction::BfCs  { offset: 40 },
        Instruction::BfCc  { offset: 44 },
        Instruction::BfVs  { offset: 48 },
        Instruction::BfVc  { offset: 52 },
        Instruction::BfEs  { offset: 56 },
        Instruction::BfEc  { offset: 60 },

        // Register branch / BC-type
        Instruction::BEq  { ra: 1, rb: 2, offset: -4 },
        Instruction::BNe  { ra: 1, rb: 2, offset: 4 },
        Instruction::BLt  { ra: 1, rb: 2, offset: 8 },
        Instruction::BLe  { ra: 1, rb: 2, offset: 12 },
        Instruction::BGt  { ra: 1, rb: 2, offset: 16 },
        Instruction::BGe  { ra: 1, rb: 2, offset: 20 },
        Instruction::BLtu { ra: 1, rb: 2, offset: 24 },
        Instruction::BLeu { ra: 1, rb: 2, offset: 28 },
        Instruction::BGtu { ra: 1, rb: 2, offset: 32 },
        Instruction::BGeu { ra: 1, rb: 2, offset: 36 },

        // PC-relative jump / call
        Instruction::Jmp  { offset: -4 },
        Instruction::Call { offset: 4 },

        // Register jump / call
        Instruction::Jr { target: 1 },
        Instruction::Jalr { rd: 15, target: 1 },
    ]
}

#[test]
fn known_good_instructions_encode_decode_round_trip() {
    for instruction in known_good_instructions() {
        let word = encode(instruction)
            .unwrap_or_else(|err| panic!("failed to encode {instruction:?}: {err:?}"));

        let decoded = decode(word).unwrap_or_else(|err| {
            panic!("failed to decode 0x{word:08X} from {instruction:?}: {err:?}")
        });

        assert_eq!(
            decoded, instruction,
            "round trip changed instruction; word=0x{word:08X}"
        );
    }
}

#[test]
fn known_good_words_decode_encode_round_trip() {
    for instruction in known_good_instructions() {
        let word = encode(instruction).unwrap_or_else(|err| {
            panic!("failed to encode seed instruction {instruction:?}: {err:?}")
        });

        let decoded = decode(word)
            .unwrap_or_else(|err| panic!("failed to decode seed word 0x{word:08X}: {err:?}"));

        let encoded = encode(decoded)
            .unwrap_or_else(|err| panic!("failed to re-encode {decoded:?}: {err:?}"));

        assert_eq!(
            encoded, word,
            "round trip changed encoded word; decoded={decoded:?}"
        );
    }
}
