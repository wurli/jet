# Ark `--install` command panics

> <https://github.com/posit-dev/ark/issues/648>
> 
> * Author: @jmcphers
> * State: CLOSED
> * Labels: 


To repro, just run `ark --install`. This appears:

```
thread 'main' panicked at /home/jmcphers/git/ark/crates/harp/src/command.rs:33:42:
called `Result::unwrap()` on an `Err` value: NotPresent
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

Some debugging suggests the problem is that this command requires `R_HOME` to be set. So you can do this:

```bash
export R_HOME=$(R RHOME)
```

After this you can install, but you get a spurious error about needing a connection file.

```
./ark --install
Successfully installed Ark Jupyter kernelspec.

    Kernel: /home/jmcphers/.local/share/jupyter/kernels/ark/kernel.json
    
Error: A connection file must be specified. Use the `--connection_file` argument.
```

It's probably fine to require R_HOME to be set so that Ark is registered against the right version of R, but we should check and see if it's set and show something friendly if it isn't. 

## @fithisux at 2024-12-14T08:25:02Z

Same on windows 10 x64, the kernel appears in Jupyter lab but cannot run anything.

But R is in the PATH and R_HOME is set correctly. @jmcphers 

PATH is `c:\winoss\R\bin`

IRKernel works fine in Jupyter lab.

Positron works fine on the other hand

## @aaelony at 2025-01-21T20:06:45Z

On Pop!Os (see #671), it seems ark cannot find R_HOME.  Within `crates/ark`, `cargo run -- --install` outputs the following message:

```
thread 'main' panicked at crates/ark/src/main.rs:387:36:
called `Result::unwrap()` on an `Err` value: Failed to execute R to determine R_HOME
```

Strangely, I see the following:
```
$ echo $R_HOME
WARNING: ignoring environment value of R_HOME /usr/lib/R


## @aaelony at 2025-01-21T20:22:12Z

On linux, the following might be useful for `harp::r_version`: 
```
R --version | grep "R version" | cut -d' ' -f3
4.4.2
```