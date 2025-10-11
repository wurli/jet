# Clean up list of direct dependencies with `cargo machete`

> <https://github.com/posit-dev/ark/pull/495>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

https://github.com/bnjbvr/cargo-machete is pretty good! Still lists a few macro/attribute related false positives

```
amalthea -- ./crates/amalthea/Cargo.toml:
        strum
harp -- ./crates/harp/Cargo.toml:
        ctor
        serde
ark -- ./crates/ark/Cargo.toml:
        ctor
        strum
```

