# Show available commands
default:
	just --list

# Build all LaTeX documentation
docs:
	./scripts/build-docs.sh
	
check-isa:
    python3 scripts/check-isa.py spec/isa/viv32-isa.yaml

gen-isa-rust:
    python3 scripts/gen-isa-rust.py spec/isa/viv32-isa.yaml emulator/src/isa/generated.rs

check-emulator:
	cd emulator && cargo check
	
isa: check-isa gen-isa-rust check-emulator
	
# Run all tests
test:
	./scripts/test-all.sh

# Remove generated files
clean:
	rm -rf build dist target obj_dir
	find docs -type f \( \
		-name "*.aux" -o \
		-name "*.log" -o \
		-name "*.toc" -o \
		-name "*.out" -o \
		-name "*.fls" -o \
		-name "*.fdb_latexmk" \
	\) -delete
