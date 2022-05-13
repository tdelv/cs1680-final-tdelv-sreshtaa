.PHONY: clean

.DEFAULT_GOAL := build

build:
	cargo build --release
	cp target/release/snowcast_control target/release/snowcast_listener target/release/snowcast_server .

debug:
	cargo build
	cp target/debug/snowcast_control target/debug/snowcast_listener target/debug/snowcast_server .

clean:
	rm -r target
	rm snowcast_control snowcast_listener snowcast_server
