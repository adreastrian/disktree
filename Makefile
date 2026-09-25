# disktree: build, bundle, check, and install.
#
# `make install` builds a release binary, wraps it in Disktree.app and copies
# the bundle to /Applications, so disktree has its icon and name, shows up in
# Spotlight and in Finder's "Open With" for folders. It needs no root on a
# normal Mac, where /Applications is writable by the user; otherwise it goes
# to ~/Applications instead and says so.
#
#   make install       # /Applications/Disktree.app
#   make install-cli   # plus a `disktree` symlink on PATH
#   make dist          # the release zip and its checksum, in dist/
#   make release       # publish dist/ as the GitHub release for the tag

# rustup puts cargo in ~/.cargo/bin, which a shell started outside a login
# profile (an editor's run button, launchd) does not have on PATH. The
# exported PATH covers the scripts; CARGO is resolved here because make runs
# a one-word recipe without a shell and searches its own startup PATH.
export PATH := $(HOME)/.cargo/bin:$(PATH)
CARGO ?= $(or $(shell command -v cargo),$(HOME)/.cargo/bin/cargo)
MANIFEST = Cargo.toml
# Where cargo puts the build: the default target/, or CARGO_TARGET_DIR if
# the environment moved it. The scripts read the same variable.
TARGET_DIR = $(or $(CARGO_TARGET_DIR),target)
TARGET = $(TARGET_DIR)/release/disktree
BUNDLE = $(TARGET_DIR)/release/Disktree.app
APPDIR = /Applications
CLI_DIR ?= /usr/local/bin
ICON = assets/disktree.icns
NOTES = packaging/release-notes.md

.PHONY: help build bundle run open install install-cli uninstall \
        lint test ci fmt clean icon dist release

help:
	@echo "disktree"
	@echo
	@echo "  make build        release build"
	@echo "  make bundle       Disktree.app in $(TARGET_DIR)/release"
	@echo "  make run          build and run the binary, scanning $$HOME"
	@echo "  make open         build, bundle and open Disktree.app"
	@echo "  make install      copy Disktree.app to /Applications"
	@echo "  make install-cli  symlink disktree into $(CLI_DIR)"
	@echo "  make uninstall    remove what install put there"
	@echo "  make dist         release zip and .sha256 in dist/"
	@echo "  make release      gh release create from dist/ for the tag"
	@echo "  make lint         rustfmt --check and clippy -D warnings"
	@echo "  make test         core and window-harness tests"
	@echo "  make ci           lint, then test"
	@echo "  make fmt          format in place"
	@echo "  make icon         re-render assets/disktree.icns from the SVG"
	@echo "  make clean        cargo clean"

# Always ask cargo: it is incremental and knows every source file, where a
# make file-target would only compare the binary against the manifest and
# happily install a stale build.
build:
	$(CARGO) build --release

bundle: build
	sh packaging/bundle.sh

# The bare binary, so stdout and panics land in this terminal; `make open`
# is the bundle, the way a user launches it.
run: build
	$(TARGET)

open: bundle
	open $(BUNDLE)

lint:
	$(CARGO) xtask lint

test:
	$(CARGO) xtask test

ci: lint test

fmt:
	$(CARGO) xtask fmt-fix

# ditto keeps the bundle's signature and resource forks intact where cp -r
# would not, and replaces an existing copy in one go. Remove the old bundle
# first so a stale file inside it never survives an upgrade.
install: bundle
	@dest=$(APPDIR); \
	if [ ! -w "$$dest" ]; then \
	    dest=$$HOME/Applications; \
	    echo "note: $(APPDIR) is not writable; installing to $$dest"; \
	    mkdir -p "$$dest"; \
	fi; \
	rm -rf "$$dest/Disktree.app" && \
	ditto $(BUNDLE) "$$dest/Disktree.app" && \
	echo && echo "installed: $$dest/Disktree.app" && \
	echo "  open it from Launchpad, Spotlight, or: open -a Disktree ~/src" && \
	echo "  make install-cli puts a 'disktree' command on PATH"

# A symlink, so the command and the app are always the same build. The dir
# may not exist or be writable on a fresh Mac; say what to do rather than
# ask for sudo from a Makefile.
install-cli:
	@app=$(APPDIR)/Disktree.app; \
	[ -d "$$app" ] || app=$$HOME/Applications/Disktree.app; \
	if [ ! -d "$$app" ]; then \
	    echo "no Disktree.app installed; run make install first" >&2; exit 1; \
	fi; \
	if [ -d $(CLI_DIR) ] && [ -w $(CLI_DIR) ]; then \
	    ln -sfn "$$app/Contents/MacOS/disktree" $(CLI_DIR)/disktree && \
	    echo "linked: $(CLI_DIR)/disktree -> $$app/Contents/MacOS/disktree"; \
	else \
	    echo "$(CLI_DIR) is not writable. Either:"; \
	    echo "  sudo mkdir -p $(CLI_DIR) && sudo ln -sfn \"$$app/Contents/MacOS/disktree\" $(CLI_DIR)/disktree"; \
	    echo "  make install-cli CLI_DIR=\$$HOME/bin      # a dir on your PATH"; \
	fi

uninstall:
	rm -rf $(APPDIR)/Disktree.app $(HOME)/Applications/Disktree.app
	@if [ -L $(CLI_DIR)/disktree ]; then rm -f $(CLI_DIR)/disktree; fi
	@echo "removed"

# The release is built here, on Apple silicon; dist.sh names the zip after
# the version in Cargo.toml and the target, so the file a user downloads
# says what it is.
dist: bundle
	sh packaging/dist.sh

# A thin wrapper around gh: the tag v<version> must already exist and point
# at HEAD, so what is published is what is tagged. Nothing here tags or
# pushes; do that first, then `make release`.
release: dist
	@version=$$(sed -n 's/^version = "\(.*\)"/\1/p' $(MANIFEST) | head -1); \
	tag=v$$version; \
	if ! git rev-parse -q --verify "refs/tags/$$tag" >/dev/null; then \
	    echo "no tag $$tag; Cargo.toml says $$version" >&2; exit 1; \
	fi; \
	if [ "$$(git rev-parse "$$tag^{commit}")" != "$$(git rev-parse HEAD)" ]; then \
	    echo "tag $$tag does not point at HEAD" >&2; exit 1; \
	fi; \
	gh release create "$$tag" dist/Disktree-$$version-*.zip \
	    dist/Disktree-$$version-*.zip.sha256 \
	    --title "disktree $$version" --notes-file $(NOTES)

# The .icns is committed so install never needs this; run it after editing
# the SVG.
icon:
	sh packaging/icon.sh

clean:
	$(CARGO) clean
