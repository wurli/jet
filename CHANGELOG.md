# Changelog

## 0.0.2

CLI
* Adds `jet --version`

Lua
* Adds `jet.version()`

CI
* Uses `cargo-dist` for distribution. Includes a neat shell script for
  installation!

Misc
* Adds a changelog

Neovim plugin
* Adds some basic stuff to download the cli/lib from GitHub releases. Still
  WIP!


## 0.0.1 – first dev release 🎉

Jet is a command line tool for interacting with Jupyter kernels. In particular,
Jet provides a REPL which allows multiple clients (e.g. you and an agent) to
connect to the same Python/R/Julia/{your favourite interpreted language} and
run code, inspect the environment, etc. A super cool application of this is
that when using a LLM for data-oriented work, your LLM can just jump into a
session with all the necessary context pre-loaded, which can remove a lot of
the context/tokens/computation needed for the LLM to reproduce your language
environment.

Jet also provides a Lua library which, among other things, lets you communicate
with kernels using raw Jupyter messages. This allows much finer grained control
over running kernels, e.g. allowing you to work with special 'comm' channels
exposed by some kernels. E.g. the Ark R kernel exposes a comm which starts an
LSP server. [`jet.ark`](https://github.com/wurli/jet.ark) is a nvim plugin
which uses this mechanism to show an R REPL in nvim's built-in terminal (via
Jet) and simultaneously connect nvim to a LSP server which is aware of what's
going on in the session. It's magic!

Jet's documentation is currently rather sparse, but this will continue to be
tidied up as the project matures.
