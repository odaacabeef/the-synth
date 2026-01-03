.PHONY: build run install uninstall clean test

# Build the project in release mode
build:
	cargo build --release

# Run the project
run:
	cargo run -- --config example-config.yaml

# List all available MIDI input and audio output devices
list:
	cargo run -- --list

# Install to ~/.cargo/bin
install:
	cargo install --path .

# Uninstall from ~/.cargo/bin
uninstall:
	cargo uninstall the-synth

# Clean build artifacts
clean:
	cargo clean

# Run tests
test:
	cargo test
