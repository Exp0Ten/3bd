root = $(shell git rev-parse --show-toplevel)
source = $(root)/code
build = $(root)/build

all: $(build)/target/tbd

$(build)/target/tbd:
	cargo build --manifest-path $(source)/Cargo.toml --release
	cp $(source)/target/release/tbd $(build)/tbd

clean:
	rm -f $(build)/tbd
