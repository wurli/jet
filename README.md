# <p align="center">Jet ✈️</p>

Jet is command-line tool and Lua library for working with Jupyter kernels,
built with love using Rust.

> [!Note]
> Jet is currently in alpha. It works pretty well but is in active development
> and **will** undergo breaking changes. Use at your own risk!

## Installation

Mac/Linux:

```
curl -LsSf https://github.com/wurli/jet/releases/latest/download/jet-installer.sh | sh
```

This will download the Jet CLI binary for your system and add it to the
`$PATH`, but you should read the script first anyway!

Jet is not yet supported on Windows. Contributions are welcome!

## CLI Usage

* `jet start [kernelspec?]`: Start a REPL powered by a given kernel. If
  `kernelspec` is not provided, Jet finds available kernels and asks you to
  choose one.

* `jet attach [session-id?]`: Join an existing session. If `session-id` is not
  provided, jet asks you to pick from sessions which are running in the current
  directory.

* `jet execute [session-id] [code]`: Run `code` in a given session, streaming
  results to stdout. You can find the `session-id` using `jet list-sessions`.
  Hint: LLMs can do a lot with this command!

* `jet skill`: Print text which can be used in a `SKILL.md` file to teach
  agents how to use Jet.

The Jet CLI provides a myriad of other handy subcommands. Use `jet -h` to see
the full list 💫

## Lua API

Jet also provides a Lua library which, among other things, lets you communicate
with kernels using raw Jupyter messages. This allows much finer grained control
over running kernels, e.g. allowing you to work with special 'comm' channels
which enable special functionality in some kernels. E.g. the [Ark R
kernel](https://github.com/posit-dev/ark) exposes a comm which starts a LSP
server. [jet.ark](https://github.com/wurli/jet.ark) is a nvim plugin which uses
this mechanism to show an R REPL in nvim's built-in terminal (via Jet) and
simultaneously connect nvim to a LSP server which is aware of what's going on
in the session. It's magic!

## jet.nvim

Currently the WIP Neovim plugin `jet.nvim` is bundled into this project - this
will eventually be broken into its own repo.
