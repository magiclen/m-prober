EXE_x86_64 = ./target/x86_64-unknown-linux-musl/release/mprober
EXE_i686 = ./target/i686-unknown-linux-musl/release/mprober
INSTALLED_EXE = /usr/local/bin/mprober

all: x86_64 i686

x86_64: $(EXE_x86_64)

i686: $(EXE_i686)

$(EXE_x86_64): $(shell find . -type f -iname '*.rs' -o -name 'Cargo.toml' | grep -v ./target | sed 's/ /\\ /g') $(shell find ./front-end ./views -type f | sed 's/ /\\ /g')
	cargo build --release --target x86_64-unknown-linux-musl
	strip $(EXE_x86_64)

$(EXE_i686): $(shell find . -type f -iname '*.rs' -o -name 'Cargo.toml' | grep -v ./target | sed 's/ /\\ /g') $(shell find ./front-end ./views -type f | sed 's/ /\\ /g')
	cross build --release --target i686-unknown-linux-musl
	strip $(EXE_i686)

install:
	$(MAKE)
	sudo cp $(EXE_x86_64) $(INSTALLED_EXE)
	sudo chown root: $(INSTALLED_EXE)
	sudo chmod 0755 $(INSTALLED_EXE)

uninstall:
	sudo rm $(INSTALLED_EXE)

test:
	cargo test --verbose

clean:
	cargo clean
