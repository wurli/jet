# Running `ark --version` gives an error

> <https://github.com/posit-dev/ark/issues/863>
> 
> * Author: @coatless
> * State: CLOSED
> * Labels: 

Whenever I'm running from shell `ark --version` I'm getting a note on the `--connection_file` argument error:

```bash
ark --version
# Ark 0.1.195
# Error: A connection file must be specified. Use the `--connection_file` argument.
```

Though, directly engaging with `ark` or `ark --help` doesn't trigger it:

```bash
ark
# Ark 0.1.195, an R Kernel.
#
# Usage: ark [OPTIONS]
# 
# Available options:
# 
# --connection_file FILE   Start the kernel with the given JSON connection file
#                         (see the Jupyter kernel documentation for details)
# -- arg1 arg2 ...         Set the argument list to pass to R; defaults to
#                          --interactive
# --startup-file FILE      An R file to run on session startup
# --session-mode MODE      The mode in which the session is running (console, notebook, background)
# --no-capture-streams     Do not capture stdout/stderr from R
# --default-repos          Set the default repositories to use, by name:
#                          "rstudio" ('cran.rstudio.com', the default), or
#                         "posit-ppm" ('packagemanager.posit.co', subject to availability), or
#                          "none" (do not alter the 'repos' option in any way)
# --repos-conf             Set the default repositories to use from a configuration file
#                         containing a list of named repositories (`name = url`)
# --version                Print the version of Ark
# --log FILE               Log to the given file (if not specified, stdout/stderr
#                          will be used)
# --install                Install the kernel spec for Ark
# --help                   Print this help message
```

Though, I've compiled `ark` locally from the `tar.gz`:

```bash
which ark
# /opt/homebrew/bin/ark
```

