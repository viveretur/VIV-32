# TODO

## Immediate

- [ ] Implement serial RX in the emulator / device layer.
- [ ] Add interactive serial input support to demos.
- [ ] Complete `viv32-dis` target decoding so branch/jump targets resolve to symbols where available.
- [ ] Add focused assembler/linker/disassembler tests around parsing, relocation, symbol resolution, and error handling.
- [ ] Keep `just clean && just demo` working as the end-to-end acceptance test.

## ISA and Specification

- [ ] Review and complete `spec/isa/viv32-isa.yaml` as the canonical machine-readable ISA specification.
- [ ] Ensure implemented instructions, registers, exceptions, reset behaviour, and interrupt semantics match the specification.
- [ ] Document any currently implicit ABI / calling-convention decisions as they become stable.

## Toolchain

- [ ] Continue hardening the assembler.
- [ ] Continue hardening the linker.
- [ ] Complete symbol-aware disassembly in `viv32-dis`.
- [ ] Expand diagnostics / demo firmware as new platform features are added.
- [ ] Keep demos self-contained, including per-demo device configuration.

## Emulator and Device Layer

- [ ] Implement serial RX.
- [ ] Define RX-ready / interrupt behaviour and clear-on-read semantics.
- [ ] Expand VDL / device abstractions as additional virtual hardware is introduced.
- [ ] Add additional MMIO devices when they serve a concrete demo or OS/runtime need.

## C Compiler

- [ ] Define the minimum supported C subset.
- [ ] Define the compiler pipeline from C source to VIV-32 assembly/object output.
- [ ] Implement the first vertical slice through the compiler.
- [ ] Compile and run a non-trivial C program through the existing assembler, linker, and emulator.
- [ ] Grow language/runtime support incrementally from working examples.

## QEMU

- [ ] Add VIV-32 as a QEMU target.
- [ ] Reuse the ISA specification and emulator behaviour as conformance references.
- [ ] Run shared firmware / diagnostics against both the reference emulator and QEMU implementation.

## Future OS / Runtime

- [ ] Define a minimal boot/runtime environment when the ISA and device model are sufficiently stable.
- [ ] Build interactive monitor functionality as part of the future OS/runtime rather than as a standalone boot-monitor project.
- [ ] Develop a small OS/runtime incrementally from diagnostics and serial I/O support.

## Hardware

- [ ] Verilog/SystemVerilog implementation.
- [ ] Establish hardware/software conformance tests before committing to a full implementation.

## Aspirational Languages

Reintroduce language implementations as time and project needs warrant.

- [ ] Forth.
- [ ] BASIC.
- [ ] Lisp.
- [ ] Additional language experiments.
