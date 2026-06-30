# Headless mini.test runner. Mirrors the layout from
# https://nvim-mini.org/mini.nvim/TESTING.
#
# Usage:
#   make test                              # all tests
#   make test FILE=tests/test_chansend.lua # single file
#
# Env overrides:
#   JET_KERNEL_JSON   kernelspec path used by tests
#                     (default: ~/Library/Jupyter/kernels/python3/kernel.json)

export JET_KERNEL_JSON ?= $(HOME)/Library/Jupyter/kernels/python3/kernel.json

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
test: deps/mini.nvim build
	@tmp_xdg="$$(mktemp -d -t jet-minitest-xdg.XXXXXX)"; \
		trap 'rm -rf "$$tmp_xdg"' EXIT; \
		XDG_DATA_HOME="$$tmp_xdg" \
		nvim --headless --noplugin -u scripts/minimal_init.lua -c "$(MINITEST_CMD)"

.PHONY: build
build:
	cargo build >&2

deps/mini.nvim:
	@mkdir -p deps
	git clone --filter=blob:none https://github.com/nvim-mini/mini.nvim $@
