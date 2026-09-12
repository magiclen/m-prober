EXE_x86_64 = ./target/x86_64-unknown-linux-musl/release/mprober
EXE_i686 = ./target/i686-unknown-linux-musl/release/mprober
INSTALLED_EXE = /usr/local/bin/mprober

# The executable embeds the built web UI, so the Rust sources are not the only inputs.
RUST_SOURCES = $(shell find . -type f \( -iname '*.rs' -o -name 'Cargo.toml' -o -name 'Cargo.lock' -o -path './front-end/*' \) -not -path './target/*' | sed 's/ /\\ /g')
WEB_UI_SOURCES = $(shell find web-ui/src web-ui/index.html web-ui/package.json web-ui/vite.config.ts web-ui/tsconfig.json web-ui/postcss.config.mjs -type f | sed 's/ /\\ /g')

# `vite build` writes the stylesheet and the page next to this one.
BUNDLE = ./front-end/js/bundle.js

all: x86_64 i686

x86_64: $(EXE_x86_64)

i686: $(EXE_i686)

web-ui: $(BUNDLE)

$(BUNDLE): $(WEB_UI_SOURCES)
	pnpm --dir web-ui install --frozen-lockfile
	pnpm --dir web-ui run build

$(EXE_x86_64): $(BUNDLE) $(RUST_SOURCES)
	cargo build --release --target x86_64-unknown-linux-musl

$(EXE_i686): $(BUNDLE) $(RUST_SOURCES)
	cross build --release --target i686-unknown-linux-musl

install:
	$(MAKE)
	sudo cp $(EXE_x86_64) $(INSTALLED_EXE)
	sudo chown root: $(INSTALLED_EXE)
	sudo chmod 0755 $(INSTALLED_EXE)

uninstall:
	sudo rm $(INSTALLED_EXE)

test:
	cargo test --verbose
	pnpm --dir web-ui test

# The built web UI is committed and the executable embeds it, so it is not cleaned away here.
clean:
	cargo clean

.PHONY: all x86_64 i686 web-ui install uninstall test clean
