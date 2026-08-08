#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

python3 scripts/check-isa.py spec/isa/viv32-isa.yaml
python3 scripts/gen-isa-rust.py spec/isa/viv32-isa.yaml emulator/src/isa/generated.rs

(
    cd emulator
    cargo test
)
