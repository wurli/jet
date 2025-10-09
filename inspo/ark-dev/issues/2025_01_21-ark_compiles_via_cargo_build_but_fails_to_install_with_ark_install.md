# ark compiles via `cargo build` but fails to install with `ark --install`

> <https://github.com/posit-dev/ark/issues/671>
> 
> * Author: @aaelony
> * State: CLOSED
> * Labels: 




The command to install `ark` **fails** even though the executable runs after compiling from source using `cargo build`:

```
$ ./target/debug/ark
Ark 0.1.159, an R Kernel.

Usage: ark [OPTIONS]

Available options:

--connection_file FILE   Start the kernel with the given JSON connection file
                         (see the Jupyter kernel documentation for details)
-- arg1 arg2 ...         Set the argument list to pass to R; defaults to
                         --interactive
--startup-file FILE      An R file to run on session startup
--session-mode MODE      The mode in which the session is running (console, notebook, background)
--no-capture-streams     Do not capture stdout/stderr from R
--default-repos          Set the default repositories to use, by name:
                         "rstudio" ('cran.rstudio.com', the default), or
                         "posit-ppm" ('packagemanager.posit.co', subject to availability), or
                         "none" (do not alter the 'repos' option in any way)
--repos-conf             Set the default repositories to use from a configuration file
                         containing a list of named repositories (`name = url`)
--version                Print the version of Ark
--log FILE               Log to the given file (if not specified, stdout/stderr
                         will be used)
--install                Install the kernel spec for Ark
--help                   Print this help message
```

I am on `Pop!_OS 22.04 LTS` and I ran:

```
wget https://github.com/posit-dev/ark/archive/refs/tags/0.1.159.tar.gz
mkdir ark
mv 0.1.159.tar.gz ark
cd ark
cargo build
```

which ran successfully. The `BUILDING.md` file states next to run `./target/debug/ark --install`.

```
warning: `ark` (lib) generated 20 warnings
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 13s
```


```
$ RUST_BACKTRACE=1; ./target/debug/ark --install
thread 'main' panicked at crates/ark/src/main.rs:387:36:
called `Result::unwrap()` on an `Err` value: Failed to execute R to determine R_HOME

Caused by:
    Not a directory (os error 20)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

```

```
$ echo $R_HOME
/usr/bin/R
```

The reason why I need to install ark is to use an R kernel interactively in the Zed editor.

The Zed editor allows R to be used [interactively](https://zed.dev/blog/repl).  To install this interactive mode for R, the [Zed docs](https://zed.dev/docs/repl) state to use `ark --install`.  

Please advise.



## @jmcphers at 2025-01-21T19:57:16Z

I think this is the same as https://github.com/posit-dev/ark/issues/648. 

## @aaelony at 2025-01-21T20:01:09Z

I will close this issue and comment further at #648.