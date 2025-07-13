root = $(shell git rev-parse --show-toplevel)
source = $(root)/code
build = $(root)/build
rust_files = $(shell find $(source)/src/*.rs)
rustc_target = x86_64-unknown-linux-gnu 		# rustc --print target-list for all

all: $(build)/tbd

$(build)/tbd: $(build)/tmp/main.elf $(build)/tmp/worker.elf
	mkdir -p $(build)/target
	$(build)/sign.py $(build)/tmp/main.elf > $(build)/tmp/sign.bin
	cp $(build)/tmp/main.elf $(build)/tmp/tbd
	cat $(build)/tmp/worker.elf >> $(build)/tmp/tbd
	cat $(build)/tmp/sign.bin >> $(build)/tmp/tbd
	cp $(build)/tmp/tbd $(build)/target/tbd

$(build)/tmp/main.elf: $(rust_files)
	@mkdir -p $(build)/tmp
	cargo build --manifest-path $(source)/Cargo.toml --release
	cp $(source)/target/release/tbd $(build)/tmp/main.elf

$(build)/tmp/worker.elf: $(source)/worker/worker.c
	@mkdir -p $(build)/tmp
	gcc -Os -o $(build)/tmp/worker.elf $(source)/worker/worker.c

clean:
	rm -rf $(build)/tmp
