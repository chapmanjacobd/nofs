.PHONY: all fmt lint test build clean check clippy install install-completions install-manpage manpage completions

all: fmt lint test build

fmt:
	cargo fmt --all

lint:
	cargo fix --broken-code --allow-dirty
	cargo clippy --fix --allow-dirty
	cargo clippy --all-targets --all-features -- -D warnings

clippy:
	cargo clippy --all-targets --all-features

test:
	cargo test --all-targets --all-features --quiet

build:
	cargo build --all-targets --all-features

check:
	cargo check --all-targets --all-features

clean:
	cargo clean

install:
	cargo install --path .

completions: build
	mkdir -p completions
	./target/debug/nofs completions bash > completions/nofs.bash
	./target/debug/nofs completions zsh > completions/nofs.zsh
	./target/debug/nofs completions fish > completions/nofs.fish
	./target/debug/nofs completions elvish > completions/nofs.elvish
	./target/debug/nofs completions powershell > completions/nofs.ps1

manpage: build
	mkdir -p man
	./target/debug/nofs manpage > man/nofs.1

install-completions: completions
	install -d $(DESTDIR)/usr/share/bash-completion/completions
	install -m 644 completions/nofs.bash $(DESTDIR)/usr/share/bash-completion/completions/nofs
	install -d $(DESTDIR)/usr/share/zsh/site-functions
	install -m 644 completions/nofs.zsh $(DESTDIR)/usr/share/zsh/site-functions/_nofs
	install -d $(DESTDIR)/usr/share/fish/vendor_completions.d
	install -m 644 completions/nofs.fish $(DESTDIR)/usr/share/fish/vendor_completions.d/nofs.fish

install-manpage: manpage
	install -d $(DESTDIR)/usr/share/man/man1
	install -m 644 man/nofs.1 $(DESTDIR)/usr/share/man/man1/nofs.1

release:
	cargo release --execute --no-confirm
