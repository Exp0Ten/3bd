root = $(shell git rev-parse --show-toplevel)
source = $(root)/code
build = $(root)/build
rust_files = $(shell find $(source)/src/*.rs)

all: $(build)/target/tbd

$(build)/target/tbd: $(rust_files)
	@mkdir -p $(build)/target
	cargo build --manifest-path $(source)/Cargo.toml --release
	cp $(source)/target/release/tbd $(build)/target/tbd

clean:
	rm -rf $(build)/target
