# VIV-32 Machine-Readable ISA Specification

This directory contains the machine-readable description of the VIV-32 instruction set architecture.

The human-readable architecture specification is the normative design reference. The YAML file in this directory captures the same encoding information in a form intended for tools.

## Files

- `viv32-isa.yaml` — machine-readable ISA metadata, instruction formats, opcode classes, condition codes, control registers, status-register bits, system operation codes, and exception causes.

## Intended Uses

The ISA description is intended to support:

- assembler encoding tables;
- disassembler decoding tables;
- reference emulator instruction decoding;
- QEMU target development;
- documentation table generation;
- instruction legality checks;
- generated ISA tests.

## Scope

The initial version focuses on encoding-level information. Full instruction semantics may be added later once the assembler and emulator structure are clearer.

The current file describes:

- architectural metadata;
- general-purpose register encodings;
- control-register encodings;
- status-register bit assignments;
- instruction format layouts;
- major opcode assignments;
- branch condition codes;
- memory access size encodings;
- system/control operation codes;
- exception cause codes.

## Validation Goals

A validator should eventually check that:

- opcode values are unique;
- format field ranges do not overlap incorrectly;
- register numbers fit in 4 bits;
- control-register numbers fit in 4 bits;
- condition codes fit in 4 bits;
- system operation codes fit in 6 bits;
- reserved ranges do not overlap assigned values;
- every instruction references a known format and opcode class;
- generated documentation tables match the architecture manual.

## Status

VIV-32 v0.1 is provisional. This file may change as the assembler, emulator, and hardware implementation are developed.
