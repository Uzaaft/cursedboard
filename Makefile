.PHONY: macos linux

macos:
	cargo build --manifest-path=macos/Cargo.toml

linux:
	echo "unimplemented"
