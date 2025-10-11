# use `spawn!` to name test threads

> <https://github.com/posit-dev/ark/pull/28>
>
> * Author: @romainfrancois
> * State: MERGED
> * Labels:

addresses https://github.com/rstudio/positron/issues/92

(this was mostly already done, except for these two test cases).

cc @DavisVaughan this has reformatted one the touched files, not sure why https://github.com/posit-dev/amalthea/pull/11 did not. Did it miss the `ark/src/r/` directory ?

## @DavisVaughan at 2023-06-12T13:40:56Z

I think the entire `crates/ark/src/r` folder can be deleted? From what I can tell, this became `harp` and isn't being used at all anymore.

It seems like the formatter looks for `lib.rs`, `main.rs`, and `tests/` and will format those and all their dependencies (like if you use `pub mod` in a lib file)

```
davis@daviss-mbp-2 amalthea % cargo +nightly fmt -v
[proc-macro (2021)] "/Users/davis/Desktop/programming/positron/positron/extensions/positron-r/amalthea/crates/amalthea/amalthea-macros/src/lib.rs"
[lib (2021)] "/Users/davis/Desktop/programming/positron/positron/extensions/positron-r/amalthea/crates/amalthea/src/lib.rs"
[test (2021)] "/Users/davis/Desktop/programming/positron/positron/extensions/positron-r/amalthea/crates/amalthea/tests/client.rs"
[lib (2021)] "/Users/davis/Desktop/programming/positron/positron/extensions/positron-r/amalthea/crates/ark/src/lib.rs"
[bin (2021)] "/Users/davis/Desktop/programming/positron/positron/extensions/positron-r/amalthea/crates/ark/src/main.rs"
[test (2021)] "/Users/davis/Desktop/programming/positron/positron/extensions/positron-r/amalthea/crates/ark/tests/environment.rs"
[bin (2021)] "/Users/davis/Desktop/programming/positron/positron/extensions/positron-r/amalthea/crates/echo/src/main.rs"
[proc-macro (2021)] "/Users/davis/Desktop/programming/positron/positron/extensions/positron-r/amalthea/crates/harp/harp-macros/src/lib.rs"
[lib (2021)] "/Users/davis/Desktop/programming/positron/positron/extensions/positron-r/amalthea/crates/harp/src/lib.rs"
[lib (2021)] "/Users/davis/Desktop/programming/positron/positron/extensions/positron-r/amalthea/crates/stdext/src/lib.rs"
rustfmt --edition 2021 /Users/davis/Desktop/programming/positron/positron/extensions/positron-r/amalthea/crates/amalthea/amalthea-macros/src/lib.rs /Users/davis/Desktop/programming/positron/positron/extensions/positron-r/amalthea/crates/amalthea/src/lib.rs /Users/davis/Desktop/programming/positron/positron/extensions/positron-r/amalthea/crates/amalthea/tests/client.rs /Users/davis/Desktop/programming/positron/positron/extensions/positron-r/amalthea/crates/ark/src/lib.rs /Users/davis/Desktop/programming/positron/positron/extensions/positron-r/amalthea/crates/ark/src/main.rs /Users/davis/Desktop/programming/positron/positron/extensions/positron-r/amalthea/crates/ark/tests/environment.rs /Users/davis/Desktop/programming/positron/positron/extensions/positron-r/amalthea/crates/echo/src/main.rs /Users/davis/Desktop/programming/positron/positron/extensions/positron-r/amalthea/crates/harp/harp-macros/src/lib.rs /Users/davis/Desktop/programming/positron/positron/extensions/positron-r/amalthea/crates/harp/src/lib.rs /Users/davis/Desktop/programming/positron/positron/extensions/positron-r/amalthea/crates/stdext/src/lib.rs
```

So I think this tells me that whole `r/` folder isn't being used when compiling the project.

I would revert your change in the `r/exec.rs` file for now, and then we can take a closer look at deleting it?

## @romainfrancois at 2023-06-12T13:52:17Z

Thanks !
