---
name: jet
description: |
    Interact with Jupyter kernels (IPython, R (Ark), Julia, etc), including
    ones which the user is working in.
---

## Basic Use

Jet is a command line tool which can be used to interact with running Jupyter
kernels.

```
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
  skill          Print the bundled agent skill documentation (SKILL.md) to stdout
  help           Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

### Running code in an existing session

For example, you can run code in the user's kernel session (and see  the
result) using `jet execute`:

```
# list-sessions show the sessions running in the current directory
❯ jet list-sessions
2026-07-03_152521_r_dotfiles_bc9832  Ark R Kernel  2026-07-03T14:25:21Z
```

```
# pass the session id to `jet execute` to run code in the user's session.
# Make sure you use --session-name so the user can see who's running code in their session!
❯ jet execute 2026-07-03_152521_r_dotfiles_bc9832 'print("hi")' --session-name Claude
[1] "hi"

# You can also pipe into `jet execute`:
echo 'print("HI")' | jet execute 2026-07-03_152521_r_dotfiles_bc9832 --session-name Claude
```

### Starting a new persistent session

Use `jet start --persist` to start a new long-lived session. Once started, you
can interact with the kernel using `jet execute`.

``` sh
# Find available kernels
jet list-kernels
/Users/JACOB.SCOTT1/.local/share/nvim/jet/kernels/ark/kernel.json
/Users/JACOB.SCOTT1/Library/Jupyter/kernels/python3/kernel.json
/Users/JACOB.SCOTT1/Library/Jupyter/kernels/ark_test/kernel.json

# Start a long-lived session
jet start \
  /Users/JACOB.SCOTT1/.local/share/nvim/jet/kernels/ark/kernel.json \
  --persist \
  --session-name Claude
```

## Advanced  use

Pass `-h` to subcommands for more docs, e.g. `jet execute -h`.

