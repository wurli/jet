# Rerun ark's `build.rs` on changes to `src/` or `resources/`

> <https://github.com/posit-dev/ark/pull/777>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

cargo is conservative by default and generally will always rerun the build script if any file in the project changes.

But if you add your own `rerun-if-changed`, then it will start to get more specific and will only run in those specific cases you specified!

We recently added `println!("cargo:rerun-if-changed=src/debug.c");` which means that `build.rs` will now _only_ rerun when that one file is changed, making `.ps.ark.version()` basically useless.

Unfortunately `debug.c` is not recognized by cargo as being something that triggers a rebuild if we just remove this, so we need to include it. So instead we broaden the scope to anything in `src` or `resources`, which seems like itll work well.

https://doc.rust-lang.org/cargo/reference/build-scripts.html#:~:text=in%20the%20FAQ.-,cargo%3A%3Arerun%2Dif%2Dchanged%3DPATH,if%20the%20file%20has%20changed

