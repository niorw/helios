# NOTE: If you change BINARY / INSTALL_DIR here,
#       also update src/config.rs to keep them in sync.

BINARY := $(shell grep '^name' Cargo.toml | head -1 | sed 's/.*= *"\(.*\)".*/\1/')
INSTALL_DIR := /usr/local/bin

.PHONY: build install uninstall clean test run

build:
	cargo build --release

install: build
	@echo "Installing $(BINARY) to $(INSTALL_DIR)..."
	@sudo cp target/release/$(BINARY) $(INSTALL_DIR)/$(BINARY)
	@sudo chmod +x $(INSTALL_DIR)/$(BINARY)
	@echo "✅ $(BINARY) installed. Run: $(BINARY) --help"

uninstall:
	@echo "Removing $(BINARY) from $(INSTALL_DIR)..."
	@sudo rm -f $(INSTALL_DIR)/$(BINARY)
	@echo "✅ $(BINARY) removed."

clean:
	cargo clean

test:
	cargo test

run:
	cargo run
