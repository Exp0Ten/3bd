all: example

example: src/main.rs src/display.rs
	cargo build
	cp target/debug/rust example

clean:
	rm -rf target
	rm -f example

