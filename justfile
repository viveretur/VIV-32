# Show available commands
default:
	just --list

# Build all LaTeX documentation
docs:
	./scripts/build-docs.sh

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
