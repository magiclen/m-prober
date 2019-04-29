EXE = ./target/x86_64-unknown-linux-musl/release/mprober
INSTALLED_EXE = /usr/local/bin/mprober

all: $(EXE)

$(EXE): $(shell find . -type f -iname '*.rs' -o -name 'Cargo.toml' | sed 's/ /\\ /g')
	cargo build --release --target x86_64-unknown-linux-musl
	strip $(EXE)
	
install:
	$(MAKE)
	sudo cp $(EXE) $(INSTALLED_EXE)
	sudo chown root. $(INSTALLED_EXE)
	sudo chmod 0755 $(INSTALLED_EXE)

uninstall:
	sudo rm $(INSTALLED_EXE)

test:
	cargo test --verbose

clean:
	cargo clean
