---
name: jet
description: |
    Interact with Jupyter kernels (IPython, R (Ark), Julia, etc), including
    ones which the user is working in. Use jet, e.g. if the user asks you to
    run code/get values/view state/do anything else in their Python/R/other
    interpreted language session.
---

## Basic Use

Jet is a command line tool which can be used to interact with running Jupyter
kernels.

Note that Jet will affect interactive computing environments that the user is
currently working in. Make sure you set
`JET_SESSION_NAME=<claud/opus/codex/other_agent>` (this will be included
against any input you send/output that results).

```
❯ jet -h
A Jupyter Kernel REPL Driver

Usage: jet <COMMAND>

Commands:
  start          Spawn a Jupyter kernel and open a REPL on it
  attach         Attach a REPL to a kernel that's already running, identified by its connection
                 file. The kernel keeps running after you exit
  list-sessions  List Jupyter sessions tracked by jet
  list-kernels   List Jupyter kernels discoverable on disk
  stop           Stop a running kernel
  show           Show a session's metadata alongside its kernelspec
  execute        Execute code against a running kernel and stream the result to stdout. Exits once
                 the kernel goes idle for the request
  send           Send code to a running kernel and exit immediately. Output (if any) is discarded —
                 the kernel runs the cell after `jet` has gone. Same target shape as `jet execute`,
                 minus rendering options
  help           Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

For example, you can run code in the user's kernel session (and see  the
result) using `jet execute`:

```
# list-sessions show the sessions running in the current directory
❯ JET_SESSION_NAME=claude jet list-sessions
2026-07-03_152521_r_dotfiles_bc9832  Ark R Kernel  2026-07-03T14:25:21Z
```

```
# pass the session id to `jet execute` to run code in the user's session:
❯ JET_SESSION_NAME=claude jet execute 2026-07-03_152521_r_dotfiles_bc9832 'print("hi")'
[1] "hi"
```

## Advanced  use

Pass `-h` to subcommands for more docs, e.g. `jet execute -h`.

