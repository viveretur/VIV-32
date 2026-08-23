
set dotenv-load := false

ROOT := justfile_directory()

# Show available commands
default:
	just --list

# Build all LaTeX documentation
docs:
	cd {{ROOT}} && ./scripts/build-docs.sh
	
check-isa:
    cd {{ROOT}} && python3 scripts/check-isa.py spec/isa/viv32-isa.yaml

gen-isa-rust:
    cd {{ROOT}} && python3 scripts/gen-isa-rust.py spec/isa/viv32-isa.yaml toolchain/crates/viv32-isa/src/spec.rs

check-emulator:
	cd {{ROOT}}/emulator && cargo check
	
isa: check-isa gen-isa-rust check-emulator
	
# Run all tests
test-all:
	cd {{ROOT}} && ./scripts/test-all.sh

demo name="hello_world" args="":
	@cd {{ROOT}} && make --no-print-directory -C demos/{{name}} run EMU_ARGS="{{args}}"

demo-disasm name="hello_world":
	cd {{ROOT}} && make -C demos/{{name}} disasm

# Remove generated files
clean:
	cd {{ROOT}} && rm -rf build dist target obj_dir
	cd {{ROOT}} && rm -rf demos/build
	cd {{ROOT}} && find docs -type f \( \
		-name "*.aux" -o \
		-name "*.log" -o \
		-name "*.toc" -o \
		-name "*.out" -o \
		-name "*.fls" -o \
		-name "*.fdb_latexmk" \
	\) -delete
