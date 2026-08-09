#!/usr/bin/env python3
"""
Generate Rust ISA constants from spec/isa/viv32-isa.yaml.

This generator intentionally emits constants only. It does not generate a
decoder or instruction semantics yet.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import Any

try:
    import yaml
except ImportError:  # pragma: no cover
    print("error: missing dependency: PyYAML", file=sys.stderr)
    print("install with: python3 -m pip install pyyaml", file=sys.stderr)
    sys.exit(2)


def parse_int(value: Any, context: str) -> int:
    if isinstance(value, int):
        return value

    if isinstance(value, str):
        try:
            return int(value, 0)
        except ValueError:
            raise ValueError(f"{context}: invalid integer value {value!r}") from None

    raise ValueError(f"{context}: expected integer or integer string, got {type(value).__name__}")


def rust_ident(name: str) -> str:
    """
    Convert YAML keys / labels into Rust CONSTANT_CASE identifiers.
    """
    name = name.replace("-", "_")
    name = re.sub(r"[^A-Za-z0-9_]", "_", name)
    name = re.sub(r"_+", "_", name)
    name = name.strip("_")
    if not name:
        return "UNNAMED"
    if name[0].isdigit():
        name = f"_{name}"
    return name.upper()


def rust_mod_ident(name: str) -> str:
    """
    Convert YAML keys into Rust snake_case module names.
    """
    name = name.replace("-", "_")
    name = re.sub(r"[^A-Za-z0-9_]", "_", name)
    name = re.sub(r"_+", "_", name)
    name = name.strip("_")
    if not name:
        return "unnamed"
    if name[0].isdigit():
        name = f"_{name}"
    return name.lower()


def hex_u32(value: int) -> str:
    return f"0x{value:08X}"


def hex_small(value: int) -> str:
    if value <= 0xF:
        return f"0x{value:X}"
    if value <= 0xFF:
        return f"0x{value:02X}"
    if value <= 0xFFFF:
        return f"0x{value:04X}"
    return f"0x{value:X}"


def parse_bit_range(raw: Any, context: str) -> tuple[int, int]:
    if not isinstance(raw, list) or len(raw) != 2:
        raise ValueError(f"{context}: expected [high, low] bit range")

    high = parse_int(raw[0], f"{context}[0]")
    low = parse_int(raw[1], f"{context}[1]")

    if high < low:
        raise ValueError(f"{context}: expected [high, low], got [{high}, {low}]")
    if high > 31 or low < 0:
        raise ValueError(f"{context}: bit range [{high}, {low}] outside 31..0")

    return high, low


def field_width(high: int, low: int) -> int:
    return high - low + 1


def field_mask(high: int, low: int) -> int:
    width = field_width(high, low)
    return ((1 << width) - 1) << low


def load_yaml(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        data = yaml.safe_load(handle)

    if not isinstance(data, dict):
        raise ValueError("top-level YAML document must be a mapping")

    return data


class RustWriter:
    def __init__(self) -> None:
        self.lines: list[str] = []
        self.indent = 0

    def write(self, line: str = "") -> None:
        self.lines.append(("    " * self.indent) + line if line else "")

    def open_mod(self, name: str) -> None:
        self.write(f"pub mod {name} {{")
        self.indent += 1

    def close(self) -> None:
        self.indent -= 1
        self.write("}")

    def const_u32(self, name: str, value: int, comment: str | None = None) -> None:
        if comment:
            self.write(f"/// {comment}")
        self.write(f"pub const {name}: u32 = {hex_small(value)};")

    def const_usize(self, name: str, value: int, comment: str | None = None) -> None:
        if comment:
            self.write(f"/// {comment}")
        self.write(f"pub const {name}: usize = {hex_small(value)};")

    def const_bool(self, name: str, value: bool, comment: str | None = None) -> None:
        if comment:
            self.write(f"/// {comment}")
        self.write(f"pub const {name}: bool = {'true' if value else 'false'};")

    def const_str(self, name: str, value: str, comment: str | None = None) -> None:
        escaped = value.replace("\\", "\\\\").replace('"', '\\"')
        if comment:
            self.write(f"/// {comment}")
        self.write(f'pub const {name}: &str = "{escaped}";')

    def finish(self) -> str:
        return "\n".join(self.lines) + "\n"


def emit_header(w: RustWriter, input_path: Path) -> None:
    w.write("// This file is generated from spec/isa/viv32-isa.yaml.")
    w.write("// Do not edit by hand.")
    w.write("//")
    w.write(f"// Source: {input_path}")
    w.write()
    w.write("#![allow(dead_code)]")
    w.write()


def emit_architecture(w: RustWriter, spec: dict[str, Any]) -> None:
    arch = spec.get("architecture", {})
    if not isinstance(arch, dict):
        return

    w.open_mod("architecture")

    for key in ["name", "version", "prefix", "endian"]:
        value = arch.get(key)
        if isinstance(value, str):
            w.const_str(rust_ident(key), value)

    for key in ["word_bits", "address_bits", "instruction_bits"]:
        if key in arch:
            w.const_u32(rust_ident(key), parse_int(arch[key], f"architecture.{key}"))

    for key in ["opcode"]:
        opdata = arch.get(key)
        if isinstance(opdata, dict):
            if "shift" in opdata:
                w.const_u32(f"{rust_ident(key)}_SHIFT", parse_int(opdata["shift"], f"architecture.{key}.shift"))
            if "width" in opdata:
                width = parse_int(opdata["width"], f"architecture.{key}.width")
                w.const_u32(f"{rust_ident(key)}_WIDTH", width)
                w.const_u32(f"{rust_ident(key)}_MASK", (1 << width) - 1)
        
    w.close()
    w.write()


def emit_registers(w: RustWriter, spec: dict[str, Any]) -> None:
    registers = spec.get("registers", {})
    if not isinstance(registers, dict):
        return

    gpr = registers.get("gpr", {})
    if not isinstance(gpr, dict):
        return

    w.open_mod("gpr")

    for key in ["width_bits", "count", "encoding_bits"]:
        if key in gpr:
            w.const_u32(rust_ident(key), parse_int(gpr[key], f"registers.gpr.{key}"))

    names = gpr.get("names", {})
    if isinstance(names, dict):
        w.write()
        for name, raw_value in names.items():
            w.const_usize(rust_ident(name), parse_int(raw_value, f"registers.gpr.names.{name}"))

    special = gpr.get("special", {})
    if isinstance(special, dict) and isinstance(names, dict):
        w.write()
        for alias, reg_name in special.items():
            if isinstance(reg_name, str) and reg_name in names:
                value = parse_int(names[reg_name], f"registers.gpr.names.{reg_name}")
                w.const_usize(rust_ident(alias), value, f"Alias for {reg_name}")

    w.close()
    w.write()


def emit_control_registers(w: RustWriter, spec: dict[str, Any]) -> None:
    control_registers = spec.get("control_registers", {})
    if not isinstance(control_registers, dict):
        return

    w.open_mod("creg")

    for name, entry in control_registers.items():
        if name == "reserved":
            continue
        if not isinstance(entry, dict) or "number" not in entry:
            continue

        comment = entry.get("description") if isinstance(entry.get("description"), str) else None
        value = parse_int(entry["number"], f"control_registers.{name}.number")
        w.const_usize(rust_ident(name), value, comment)

    w.close()
    w.write()


def emit_status_register(w: RustWriter, spec: dict[str, Any]) -> None:
    status = spec.get("status_register", {})
    if not isinstance(status, dict):
        return

    bits = status.get("bits", {})
    if not isinstance(bits, dict):
        return

    w.open_mod("sr")

    for name, entry in bits.items():
        if not isinstance(entry, dict) or "bit" not in entry:
            continue

        bit = parse_int(entry["bit"], f"status_register.bits.{name}.bit")
        comment = entry.get("description") if isinstance(entry.get("description"), str) else None

        ident = rust_ident(name)
        w.const_u32(f"{ident}_BIT", bit, comment)
        w.write(f"pub const {ident}_MASK: u32 = 1u32 << {ident}_BIT;")
        w.write()

    w.close()
    w.write()


def emit_formats(w: RustWriter, spec: dict[str, Any]) -> None:
    formats = spec.get("formats", {})
    if not isinstance(formats, dict):
        return

    w.open_mod("format")

    for format_name, format_body in formats.items():
        if not isinstance(format_body, dict):
            continue

        fields = format_body.get("fields", {})
        if not isinstance(fields, dict):
            continue

        mod_name = rust_mod_ident(format_name)
        w.open_mod(mod_name)

        for field_name, field_body in fields.items():
            if not isinstance(field_body, dict) or "bits" not in field_body:
                continue

            high, low = parse_bit_range(
                field_body["bits"],
                f"formats.{format_name}.fields.{field_name}.bits",
            )
            width = field_width(high, low)
            mask = field_mask(high, low)

            ident = rust_ident(field_name)
            w.const_u32(f"{ident}_SHIFT", low)
            w.const_u32(f"{ident}_WIDTH", width)
            w.write(f"pub const {ident}_MASK: u32 = {hex_u32(mask)};")
            w.write()
            
        subfields = format_body.get("subfields", {})
        if isinstance(subfields, dict):
            for subfield_name, subfield_body in subfields.items():
                if not isinstance(subfield_body, dict):
                    continue

                bit_key = "instruction_bits" if "instruction_bits" in subfield_body else "bits"

                if bit_key not in subfield_body:
                    continue

                high, low = parse_bit_range(
                    subfield_body[bit_key],
                    f"formats.{format_name}.subfields.{subfield_name}.{bit_key}",
                )
                width = field_width(high, low)
                mask = field_mask(high, low)

                ident = rust_ident(subfield_name)
                w.const_u32(f"{ident}_SHIFT", low)
                w.const_u32(f"{ident}_WIDTH", width)
                w.write(f"pub const {ident}_MASK: u32 = {hex_u32(mask)};")
                w.write()
                
        w.close()
        w.write()

    w.close()
    w.write()


def emit_opcode_constants(w: RustWriter, spec: dict[str, Any]) -> None:
    opcodes = spec.get("opcodes", {})
    if not isinstance(opcodes, dict):
        return

    w.open_mod("opcode")

    for name, entry in opcodes.items():
        if name == "reserved":
            continue
        if not isinstance(entry, dict) or "value" not in entry:
            continue

        comment = entry.get("class") if isinstance(entry.get("class"), str) else None
        value = parse_int(entry["value"], f"opcodes.{name}.value")
        w.const_u32(rust_ident(name), value, comment)

    w.close()
    w.write()


def emit_conditions(w: RustWriter, spec: dict[str, Any]) -> None:
    conditions = spec.get("conditions", {})
    if not isinstance(conditions, dict):
        return

    w.open_mod("condition")

    for name, entry in conditions.items():
        if not isinstance(entry, dict) or "value" not in entry:
            continue

        comment = entry.get("meaning") if isinstance(entry.get("meaning"), str) else None
        value = parse_int(entry["value"], f"conditions.{name}.value")
        w.const_u32(rust_ident(name), value, comment)

    w.close()
    w.write()


def emit_memory_sizes(w: RustWriter, spec: dict[str, Any]) -> None:
    memory_sizes = spec.get("memory_sizes", {})
    if not isinstance(memory_sizes, dict):
        return

    w.open_mod("memory_size")

    for name, entry in memory_sizes.items():
        if not isinstance(entry, dict) or "value" not in entry:
            continue

        value = parse_int(entry["value"], f"memory_sizes.{name}.value")
        ident = rust_ident(name)
        w.const_u32(ident, value)

        if "bits" in entry:
            bits = parse_int(entry["bits"], f"memory_sizes.{name}.bits")
            w.const_u32(f"{ident}_BITS", bits)

        if "alignment" in entry:
            alignment = parse_int(entry["alignment"], f"memory_sizes.{name}.alignment")
            w.const_u32(f"{ident}_ALIGNMENT", alignment)

        w.write()

    w.close()
    w.write()


def emit_system_ops(w: RustWriter, spec: dict[str, Any]) -> None:
    system_ops = spec.get("system_ops", {})
    if not isinstance(system_ops, dict):
        return

    w.open_mod("sysop")

    for name, entry in system_ops.items():
        if name == "reserved":
            continue
        if not isinstance(entry, dict) or "sysop" not in entry:
            continue

        value = parse_int(entry["sysop"], f"system_ops.{name}.sysop")
        w.const_u32(rust_ident(name), value)

    w.close()
    w.write()


def emit_exception_causes(w: RustWriter, spec: dict[str, Any]) -> None:
    causes = spec.get("exception_causes", {})
    if not isinstance(causes, dict):
        return

    w.open_mod("exception_cause")

    for name, entry in causes.items():
        if name == "reserved":
            continue
        if not isinstance(entry, dict) or "value" not in entry:
            continue

        comment = entry.get("description") if isinstance(entry.get("description"), str) else None
        value = parse_int(entry["value"], f"exception_causes.{name}.value")
        w.const_u32(rust_ident(name), value, comment)

    w.close()
    w.write()


def emit_function_codes(w: RustWriter, spec: dict[str, Any]) -> None:
    function_codes = spec.get("function_codes", {})
    if not isinstance(function_codes, dict):
        return

    w.open_mod("func")

    for group_name, group in function_codes.items():
        if not isinstance(group, dict):
            continue

        values = group.get("values", {})
        if not isinstance(values, dict):
            continue

        w.open_mod(rust_mod_ident(group_name))

        for name, entry in values.items():
            if name == "reserved":
                continue
            if not isinstance(entry, dict) or "value" not in entry:
                continue

            value = parse_int(entry["value"], f"function_codes.{group_name}.values.{name}.value")
            w.const_u32(rust_ident(name), value)

        w.close()
        w.write()

    w.close()
    w.write()


def emit_mode_codes(w: RustWriter, spec: dict[str, Any]) -> None:
    mode_codes = spec.get("mode_codes", {})
    if not isinstance(mode_codes, dict):
        return

    w.open_mod("mode")

    for group_name, group in mode_codes.items():
        if not isinstance(group, dict):
            continue

        values = group.get("values", {})
        if not isinstance(values, dict):
            continue

        w.open_mod(rust_mod_ident(group_name))

        for name, entry in values.items():
            if name == "reserved":
                continue
            if not isinstance(entry, dict) or "value" not in entry:
                continue

            value = parse_int(entry["value"], f"mode_codes.{group_name}.values.{name}.value")
            w.const_u32(rust_ident(name), value)

        w.close()
        w.write()

    w.close()
    w.write()


def generate(spec: dict[str, Any], input_path: Path) -> str:
    w = RustWriter()

    emit_header(w, input_path)
    emit_architecture(w, spec)
    emit_registers(w, spec)
    emit_control_registers(w, spec)
    emit_status_register(w, spec)
    emit_formats(w, spec)
    emit_opcode_constants(w, spec)
    emit_function_codes(w, spec)
    emit_mode_codes(w, spec)
    emit_conditions(w, spec)
    emit_memory_sizes(w, spec)
    emit_system_ops(w, spec)
    emit_exception_causes(w, spec)

    return w.finish()


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Generate Rust ISA constants from VIV-32 ISA YAML.")
    parser.add_argument("input", help="Path to viv32-isa.yaml")
    parser.add_argument("output", help="Path to generated Rust output")
    args = parser.parse_args(argv)

    input_path = Path(args.input)
    output_path = Path(args.output)

    try:
        spec = load_yaml(input_path)
        output = generate(spec, input_path)
    except Exception as exc:
        print(f"error: failed to generate Rust ISA constants: {exc}", file=sys.stderr)
        return 1

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(output, encoding="utf-8")

    print(f"generated {output_path} from {input_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
