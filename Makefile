# OpenDefAlgSplitting — atajos de compilación y tests
# Requiere: rustup, cargo

BINARY = opendefalgsplitting
RELEASE = target/release/$(BINARY)
DEBUG   = target/debug/$(BINARY)

.PHONY: all release debug test clean cuda windows windows-msvc linux-static help

# Por defecto: release
all: release

release:
	cargo build --release
	@echo "Binario: $(RELEASE)"

debug:
	cargo build
	@echo "Binario: $(DEBUG)"

test:
	cargo test

test-release:
	cargo test --release

clean:
	cargo clean

# Con soporte CUDA (GPU)
cuda:
	cargo build --release --features cuda
	@echo "Binario: $(RELEASE) (con CUDA)"

# Cross-compilar para Windows (desde Linux/macOS)
# Necesitas: rustup target add x86_64-pc-windows-gnu
# En Linux (Debian/Ubuntu): sudo apt install mingw-w64
windows:
	rustup target add x86_64-pc-windows-gnu 2>/dev/null || true
	cargo build --release --target x86_64-pc-windows-gnu
	@echo "Binario Windows: target/x86_64-pc-windows-gnu/release/$(BINARY).exe"

# Binario Linux estático (sin depender de glibc del sistema)
# Necesitas: rustup target add x86_64-unknown-linux-musl
# En Debian/Ubuntu: sudo apt install musl-tools
linux-static:
	rustup target add x86_64-unknown-linux-musl 2>/dev/null || true
	cargo build --release --target x86_64-unknown-linux-musl
	@echo "Binario estático: target/x86_64-unknown-linux-musl/release/$(BINARY)"

help:
	@echo "Targets:"
	@echo "  make / make release  - compilar en release (recomendado)"
	@echo "  make debug           - compilar sin optimizar"
	@echo "  make test            - ejecutar tests"
	@echo "  make clean           - borrar target/"
	@echo "  make cuda            - release con feature cuda (GPU)"
	@echo "  make windows         - .exe para Windows (desde Linux, necesita mingw-w64)"
	@echo "  make linux-static    - binario Linux estático (musl, sin dep. glibc)"
