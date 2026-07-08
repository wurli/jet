# Test entrypoints.
#
# Usage:
#   make test                              # cargo + nvim plugin tests
#   make test-cargo                        # cargo workspace tests only
#   make test-plugin                       # nvim mini.test only
#   make test-plugin FILE=tests/foo.lua    # single plugin test file
#
# Env overrides:
#   JET_KERNEL_JSON   kernelspec path used by the plugin tests
#                     (default: the repo's dev-installed python3 kernel)

# Point kernel discovery at the repo's `kernels/` dir, populated by
# `scripts/install-dev-kernels.sh`.
export JUPYTER_PATH := $(CURDIR)

export JET_KERNEL_JSON ?= $(CURDIR)/kernels/python3/kernel.json

# Prepend the cargo-built `jet` to PATH so tests find it by name. The tests
# themselves also expect an isolated XDG_DATA_HOME so they don't touch the
# user's real one — we mint a fresh tempdir per `make test` invocation.
export PATH := $(CURDIR)/target/debug:$(PATH)

FILE ?=
ifeq ($(FILE),)
	MINITEST_CMD := lua MiniTest.run()
else
	MINITEST_CMD := lua MiniTest.run_file('$(FILE)')
endif

.PHONY: test
test: test-cargo test-plugin

.PHONY: test-cargo
test-cargo:
	cargo test --workspace --no-fail-fast

.PHONY: test-plugin
test-plugin: deps/mini.nvim build
	@tmp_xdg="$$(mktemp -d -t jet-minitest-xdg.XXXXXX)"; \
		trap 'rm -rf "$$tmp_xdg"' EXIT; \
		XDG_DATA_HOME="$$tmp_xdg" \
		nvim --headless --noplugin -u scripts/minimal_init.lua -c "$(MINITEST_CMD)"

.PHONY: build
build:
	cargo build >&2

.PHONY: install-dev-kernels
install-dev-kernels:
	./scripts/install-dev-kernels.sh

deps/mini.nvim:
	@mkdir -p deps
	git clone --filter=blob:none https://github.com/nvim-mini/mini.nvim $@
