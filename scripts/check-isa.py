#!/usr/bin/env python3
"""
Validate the VIV-32 machine-readable ISA specification.

This first-pass validator checks the encoding namespaces and structural
consistency of spec/isa/viv32-isa.yaml. It does not yet validate individual
instruction records, because those have not been added yet.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path
from typing import Any

try:
    import yaml
except ImportError:  # pragma: no cover
    print("error: missing dependency: PyYAML", file=sys.stderr)
    print("install with: python3 -m pip install pyyaml", file=sys.stderr)
    sys.exit(2)


ErrorList = list[str]


def parse_int(value: Any, context: str) -> int:
    """
    Parse integers from YAML values.

    Accepts:
      - integer YAML scalars
      - strings such as "0x0A", "0b10", or "15"
    """
    if isinstance(value, int):
        return value

    if isinstance(value, str):
        try:
            return int(value, 0)
        except ValueError:
            raise ValueError(f"{context}: invalid integer value {value!r}") from None

    raise ValueError(f"{context}: expected integer or integer string, got {type(value).__name__}")


def require_mapping(value: Any, context: str, errors: ErrorList) -> dict[str, Any]:
    if not isinstance(value, dict):
        errors.append(f"{context}: expected mapping")
        return {}
    return value


def require_list(value: Any, context: str, errors: ErrorList) -> list[Any]:
    if not isinstance(value, list):
        errors.append(f"{context}: expected list")
        return []
    return value


def parse_bit_range(value: Any, context: str, errors: ErrorList) -> tuple[int, int] | None:
    items = require_list(value, context, errors)
    if len(items) != 2:
        errors.append(f"{context}: bit range must contain exactly two values")
        return None

    try:
        high = parse_int(items[0], f"{context}[0]")
        low = parse_int(items[1], f"{context}[1]")
    except ValueError as exc:
        errors.append(str(exc))
        return None

    if high < low:
        errors.append(f"{context}: expected [high, low], got [{high}, {low}]")
        return None

    if high > 31 or low < 0:
        errors.append(f"{context}: bit range [{high}, {low}] outside 31..0")
        return None

    return high, low


def get_path(root: dict[str, Any], path: str, errors: ErrorList) -> Any:
    current: Any = root
    for part in path.split("."):
        if not isinstance(current, dict) or part not in current:
            errors.append(f"{path}: missing required section")
            return {}
        current = current[part]
    return current


def check_unique_values(
    entries: dict[str, Any],
    value_key: str,
    bit_width: int,
    context: str,
    errors: ErrorList,
    skip_names: set[str] | None = None,
) -> None:
    skip_names = skip_names or set()
    seen: dict[int, str] = {}
    max_value = (1 << bit_width) - 1

    for name, entry in entries.items():
        if name in skip_names:
            continue

        if not isinstance(entry, dict):
            errors.append(f"{context}.{name}: expected mapping")
            continue

        if value_key not in entry:
            errors.append(f"{context}.{name}: missing {value_key!r}")
            continue

        try:
            value = parse_int(entry[value_key], f"{context}.{name}.{value_key}")
        except ValueError as exc:
            errors.append(str(exc))
            continue

        if value < 0 or value > max_value:
            errors.append(
                f"{context}.{name}.{value_key}: value 0x{value:X} does not fit in {bit_width} bits"
            )

        if value in seen:
            errors.append(
                f"{context}: duplicate {value_key} 0x{value:X} used by {seen[value]!r} and {name!r}"
            )
        else:
            seen[value] = name


def parse_range(value: Any, context: str, errors: ErrorList) -> tuple[int, int] | None:
    items = require_list(value, context, errors)
    if len(items) != 2:
        errors.append(f"{context}: range must contain exactly two values")
        return None

    try:
        start = parse_int(items[0], f"{context}[0]")
        end = parse_int(items[1], f"{context}[1]")
    except ValueError as exc:
        errors.append(str(exc))
        return None

    if start > end:
        errors.append(f"{context}: range start 0x{start:X} is greater than end 0x{end:X}")
        return None

    return start, end


def check_reserved_range_no_overlap(
    entries: dict[str, Any],
    value_key: str,
    reserved_name: str,
    context: str,
    errors: ErrorList,
) -> None:
    reserved = entries.get(reserved_name)
    if not isinstance(reserved, dict) or "range" not in reserved:
        return

    parsed = parse_range(reserved["range"], f"{context}.{reserved_name}.range", errors)
    if parsed is None:
        return

    start, end = parsed

    for name, entry in entries.items():
        if name == reserved_name:
            continue

        if not isinstance(entry, dict) or value_key not in entry:
            continue

        try:
            value = parse_int(entry[value_key], f"{context}.{name}.{value_key}")
        except ValueError as exc:
            errors.append(str(exc))
            continue

        if start <= value <= end:
            errors.append(
                f"{context}.{name}.{value_key}: value 0x{value:X} overlaps reserved range "
                f"0x{start:X}..0x{end:X}"
            )


def check_format_fields(spec: dict[str, Any], errors: ErrorList) -> None:
    formats = require_mapping(get_path(spec, "formats", errors), "formats", errors)

    for format_name, format_body in formats.items():
        format_body = require_mapping(format_body, f"formats.{format_name}", errors)
        fields = require_mapping(
            format_body.get("fields"),
            f"formats.{format_name}.fields",
            errors,
        )

        occupied: dict[int, str] = {}

        for field_name, field_body in fields.items():
            field_body = require_mapping(
                field_body,
                f"formats.{format_name}.fields.{field_name}",
                errors,
            )

            if "bits" not in field_body:
                errors.append(f"formats.{format_name}.fields.{field_name}: missing 'bits'")
                continue

            bit_range = parse_bit_range(
                field_body["bits"],
                f"formats.{format_name}.fields.{field_name}.bits",
                errors,
            )
            if bit_range is None:
                continue

            high, low = bit_range

            for bit in range(low, high + 1):
                if bit in occupied:
                    errors.append(
                        f"formats.{format_name}: bit {bit} used by both "
                        f"{occupied[bit]!r} and {field_name!r}"
                    )
                else:
                    occupied[bit] = field_name

        missing_bits = [bit for bit in range(32) if bit not in occupied]
        if missing_bits:
            missing = ", ".join(str(bit) for bit in missing_bits)
            errors.append(f"formats.{format_name}: bits not covered by fields: {missing}")


def check_registers(spec: dict[str, Any], errors: ErrorList) -> None:
    gpr = require_mapping(get_path(spec, "registers.gpr", errors), "registers.gpr", errors)
    names = require_mapping(gpr.get("names"), "registers.gpr.names", errors)

    seen: dict[int, str] = {}

    for name, raw_value in names.items():
        try:
            value = parse_int(raw_value, f"registers.gpr.names.{name}")
        except ValueError as exc:
            errors.append(str(exc))
            continue

        if value < 0 or value > 0xF:
            errors.append(f"registers.gpr.names.{name}: value 0x{value:X} does not fit in 4 bits")

        if value in seen:
            errors.append(
                f"registers.gpr.names: duplicate value 0x{value:X} used by {seen[value]!r} and {name!r}"
            )
        else:
            seen[value] = name

    expected_count = gpr.get("count")
    if expected_count is not None:
        try:
            count = parse_int(expected_count, "registers.gpr.count")
            if len(names) != count:
                errors.append(
                    f"registers.gpr.names: expected {count} registers, found {len(names)}"
                )
        except ValueError as exc:
            errors.append(str(exc))


def check_control_registers(spec: dict[str, Any], errors: ErrorList) -> None:
    control_registers = require_mapping(
        get_path(spec, "control_registers", errors),
        "control_registers",
        errors,
    )

    check_unique_values(
        control_registers,
        "number",
        4,
        "control_registers",
        errors,
        skip_names={"reserved"},
    )

    check_reserved_range_no_overlap(
        control_registers,
        "number",
        "reserved",
        "control_registers",
        errors,
    )


def check_status_register(spec: dict[str, Any], errors: ErrorList) -> None:
    bits = require_mapping(
        get_path(spec, "status_register.bits", errors),
        "status_register.bits",
        errors,
    )

    check_unique_values(bits, "bit", 5, "status_register.bits", errors)

    reserved = get_path(spec, "status_register.reserved", errors)
    reserved = require_mapping(reserved, "status_register.reserved", errors)

    if "range" in reserved:
        parsed = parse_range(reserved["range"], "status_register.reserved.range", errors)
        if parsed is not None:
            start, end = parsed
            for name, body in bits.items():
                if not isinstance(body, dict) or "bit" not in body:
                    continue
                try:
                    bit = parse_int(body["bit"], f"status_register.bits.{name}.bit")
                except ValueError as exc:
                    errors.append(str(exc))
                    continue

                if start <= bit <= end:
                    errors.append(
                        f"status_register.bits.{name}.bit: bit {bit} overlaps reserved range "
                        f"{start}..{end}"
                    )


def check_opcodes(spec: dict[str, Any], errors: ErrorList) -> None:
    opcodes = require_mapping(get_path(spec, "opcodes", errors), "opcodes", errors)
    formats = require_mapping(get_path(spec, "formats", errors), "formats", errors)

    check_unique_values(opcodes, "value", 6, "opcodes", errors, skip_names={"reserved"})
    check_reserved_range_no_overlap(opcodes, "value", "reserved", "opcodes", errors)

    for name, entry in opcodes.items():
        if name == "reserved":
            continue
        if not isinstance(entry, dict):
            continue

        format_name = entry.get("format")
        if not isinstance(format_name, str):
            errors.append(f"opcodes.{name}.format: missing or invalid format")
        elif format_name not in formats:
            errors.append(f"opcodes.{name}.format: unknown format {format_name!r}")


def check_conditions(spec: dict[str, Any], errors: ErrorList) -> None:
    conditions = require_mapping(get_path(spec, "conditions", errors), "conditions", errors)

    check_unique_values(conditions, "value", 4, "conditions", errors)

    if len(conditions) != 16:
        errors.append(f"conditions: expected exactly 16 condition codes, found {len(conditions)}")


def check_memory_sizes(spec: dict[str, Any], errors: ErrorList) -> None:
    memory_sizes = require_mapping(get_path(spec, "memory_sizes", errors), "memory_sizes", errors)

    check_unique_values(memory_sizes, "value", 2, "memory_sizes", errors)

    for name, entry in memory_sizes.items():
        if not isinstance(entry, dict):
            continue

        if name == "reserved":
            continue

        if "alignment" not in entry:
            errors.append(f"memory_sizes.{name}: missing alignment")
            continue

        try:
            alignment = parse_int(entry["alignment"], f"memory_sizes.{name}.alignment")
        except ValueError as exc:
            errors.append(str(exc))
            continue

        if alignment not in {1, 2, 4, 8, 16}:
            errors.append(f"memory_sizes.{name}.alignment: unusual alignment {alignment}")


def check_system_ops(spec: dict[str, Any], errors: ErrorList) -> None:
    system_ops = require_mapping(get_path(spec, "system_ops", errors), "system_ops", errors)

    check_unique_values(system_ops, "sysop", 6, "system_ops", errors, skip_names={"reserved"})
    check_reserved_range_no_overlap(system_ops, "sysop", "reserved", "system_ops", errors)


def check_exception_causes(spec: dict[str, Any], errors: ErrorList) -> None:
    causes = require_mapping(
        get_path(spec, "exception_causes", errors),
        "exception_causes",
        errors,
    )

    check_unique_values(causes, "value", 8, "exception_causes", errors, skip_names={"reserved"})
    check_reserved_range_no_overlap(causes, "value", "reserved", "exception_causes", errors)


def validate(spec: dict[str, Any]) -> ErrorList:
    errors: ErrorList = []

    check_registers(spec, errors)
    check_control_registers(spec, errors)
    check_status_register(spec, errors)
    check_format_fields(spec, errors)
    check_opcodes(spec, errors)
    check_conditions(spec, errors)
    check_memory_sizes(spec, errors)
    check_system_ops(spec, errors)
    check_exception_causes(spec, errors)

    return errors


def load_yaml(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        data = yaml.safe_load(handle)

    if not isinstance(data, dict):
        raise ValueError("top-level YAML document must be a mapping")

    return data


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Validate the VIV-32 ISA YAML specification.")
    parser.add_argument(
        "path",
        nargs="?",
        default="spec/isa/viv32-isa.yaml",
        help="Path to viv32-isa.yaml",
    )
    args = parser.parse_args(argv)

    path = Path(args.path)

    try:
        spec = load_yaml(path)
    except FileNotFoundError:
        print(f"error: file not found: {path}", file=sys.stderr)
        return 2
    except Exception as exc:
        print(f"error: failed to read {path}: {exc}", file=sys.stderr)
        return 2

    errors = validate(spec)

    if errors:
        print(f"{path}: ISA validation failed:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    print(f"{path}: ISA validation passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
