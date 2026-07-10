# <p align="center">✈️ Jet</p>

Jet is command-line tool and Lua library for working with Jupyter kernels,
built with love using Rust.

> [!Note]
> Jet is currently in alpha. It works pretty well but is in active development
> and **will** undergo breaking changes. Use at your own risk!

## CLI

Jet's CLI provides:
*  A REPL interface for running kernels, complete with kitty graphics
*  Utilities for starting/stopping kernels, identifying them on the system,
   running one-off code, etc

## Lua lib

Jet also provides a Lua library which does the same stuff but allows for more
control. For now the main consumer is the Neovim plugin `jet.nvim`.

## jet.nvim

Currently `jet.nvim` is bundled into this project - this will likely eventually
be broken into its own repo.

### Installation

Prebuilt binaries for macOS (arm64) and Linux (x86_64, arm64) are available
on the [releases page](https://github.com/wurli/jet/releases/latest).
Download the tarball for your platform, extract, and put `jet` on your `PATH`.

Jet is not yet supported on Windows. Contributions are welcome!
