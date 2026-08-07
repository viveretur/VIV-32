# TODO

## Phase 1 — Architecture Specification

- [ ] Choose final architecture name.
- [ ] Define design goals and non-goals.
- [ ] Define register file.
- [ ] Define memory model.
- [ ] Define instruction width.
- [ ] Define instruction formats.
- [ ] Define initial opcode map.
- [ ] Define reset behaviour.
- [ ] Define trap/interrupt model.
- [ ] Define halt behaviour.

## Phase 2 — Machine-Readable Spec

- [ ] Populate `spec/isa/registers.toml`.
- [ ] Populate `spec/isa/instructions.toml`.
- [ ] Populate `spec/isa/opcodes.toml`.
- [ ] Populate `spec/isa/exceptions.toml`.

## Phase 3 — Reference Emulator

- [ ] Create Rust emulator crate.
- [ ] Implement memory abstraction.
- [ ] Implement register file.
- [ ] Implement instruction fetch.
- [ ] Implement decode skeleton.
- [ ] Implement `halt`.
- [ ] Implement arithmetic instructions.
- [ ] Implement branch instructions.
- [ ] Implement load/store instructions.
- [ ] Add trace output.

## Phase 4 — Assembler / Disassembler

- [ ] Define assembly syntax.
- [ ] Implement lexer.
- [ ] Implement parser.
- [ ] Encode first instruction.
- [ ] Assemble `firmware/diagnostics/halt.S`.
- [ ] Disassemble generated binary.

## Phase 5 — Shared Tests

- [ ] Define YAML test format.
- [ ] Add `halt` ISA test.
- [ ] Add arithmetic tests.
- [ ] Add branch tests.
- [ ] Add load/store tests.
- [ ] Run tests against emulator.

## Later

- [ ] QEMU target.
- [ ] Verilog/SystemVerilog implementation.
- [ ] Boot monitor.
- [ ] Tiny OS runtime.
- [ ] Forth.
- [ ] BASIC.
- [ ] Lisp.
- [ ] C subset.# TODO:
