#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"

build_doc() {
  local dir="$1"
  local tex="$2"

  (
    cd "$repo_root/$dir"
    mkdir -p build

    latexmk -pdf -interaction=nonstopmode \
      -halt-on-error \
      -outdir=build \
      "$tex"
  )
}

build_doc "docs/architecture" "architecture.tex"
# build_doc "docs/platform" "platform.tex"
# build_doc "docs/abi" "abi.tex"
# build_doc "docs/toolchain" "toolchain.tex"
