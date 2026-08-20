# Changelog

## 0.0.7

Lua
* Lua functions which send jupyter messages to the kernel now immediately
  return the generated message id, along with any callbacks etc. This can be
  helpful if you want to detect whether messages received from the kernel are
  related to any that you previously sent, i.e. via the parent header.
* Kernel callbacks now uniformly return a value with the shape
  `{ status = "pending" | "done" } | { status = "ready", value = ... }`
* Type stubs overhauled to be slightly more principled

CLI
* Added cursed heuristics to determine which error information to use from
  Jupyter `ename`, `evalue` and `traceback`. This seems to handle both IPython
  and ark with `--session-mode notebook` or `--session-mode console`. Highly
  possible (probable?) other kernels will not fit the pattern though, in which
  case will need to revisit.

  Until now Jet has followed the JupyterLab behaviour of omitting both the
  `ename` and `evalue` if the `traceback` is present. This change breaks from
  JupyterLab behaviour for the following case:

  ``` json
  {
      "ename": "",
      "evalue": "Foo",
      "traceback": "Bar"
  }
  ```

  * JupyterLab will show only the traceback
  * Jet will show `{evalue}\n{traceback}`

## 0.0.6

Lua
* The log file can now be set using `$JET_LUA_LOG`

CLI
* The log file can now be set using `$JET_LOG`
* Fix: backspace causes the completions window to refresh

Lsp
* Completions no longer get triggered on empty lines (this was causing issues
  with IPython 7.3.0 in particular)

## 0.0.5

Lua
* Fix: `list_sessions()` is now async, but also way faster
* Fix: `jet.stop()` records closure in `session.json`

CLI
* with `--no-graphics` repl no longer prints `[image/png NxN bytes]`

Lsp
* Fix: completions no longer trigger when there's no text before the cursor
  (was causing noisy errors in some versions of ipykernel)

General
* Fix: Kernels which fail to start no longer linger as open in the session store
* Feat: better error messages when kernel command not found


## 0.0.4

* Lua: `jet.stop()` is now non-blocking
* CI now uses nextest
* `jet skill` now teaches agents how to start their own persistent Jet session
* Repl emits window title signal (OSC 0/1/2)
* Repl emits semantic prompt markers (OSC 133)
* Repl now handles `update_display_data` messages

## 0.0.3 - LSP integration 💫

* Completions no longer suck
* Completions are powered internally by a LSP server. Each Jet client spawns
  one LSP server - you can get this in Lua from the callback returned by
  `jet.start()` and/or `jet.attach()`
* jet.nvim has been split into its own repo [jet.nvim](https://github.com/wurli/jet.nvim)

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
Jet provides a repl which allows multiple clients (e.g. you and an agent) to
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
which uses this mechanism to show an R repl in nvim's built-in terminal (via
Jet) and simultaneously connect nvim to a LSP server which is aware of what's
going on in the session. It's magic!

Jet's documentation is currently rather sparse, but this will continue to be
tidied up as the project matures.
