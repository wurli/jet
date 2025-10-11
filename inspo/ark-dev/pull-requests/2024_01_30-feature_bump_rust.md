# Bump Rust requirement to 1.75.0

> <https://github.com/posit-dev/ark/pull/225>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Now that we've resolved the warnings that come with it

```
rustup update stable
```

should update, and you should see:

```
rustc --version
rustc 1.75.0 (82e1608df 2023-12-21)

rustup default
stable-aarch64-apple-darwin (default)
```

Note that we still run rustfmt with nightly builds due to needing unstable options there https://github.com/posit-dev/amalthea/pull/11

