# Remove remaining mutable static referencing warnings

> <https://github.com/posit-dev/ark/pull/216>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

It feels like I'm doing the exact same thing we were before in `&R_MAIN` but I guess this is different 🤷 

```
warning: mutable reference of mutable static is discouraged
  --> crates/harp/src/routines.rs:30:20
   |
30 |     let routines = &mut R_ROUTINES;
   |                    ^^^^^^^^^^^^^^^ mutable reference of mutable static
   |
   = note: for more information, see issue #114447 <https://github.com/rust-lang/rust/issues/114447>
   = note: reference of mutable static is a hard error from 2024 edition
   = note: mutable statics can be written to by multiple threads: aliasing violations or data races will cause undefined behavior
   = note: `#[warn(static_mut_ref)]` on by default
help: mutable references are dangerous since if there's any other pointer or reference used for that static while the reference lives, that's UB; use `addr_of_mut!` instead to create a raw pointer
   |
30 |     let routines = addr_of_mut!(R_ROUTINES);
   |                    ~~~~~~~~~~~~~~~~~~~~~~~~

warning: `harp` (lib) generated 1 warning
warning: shared reference of mutable static is discouraged
   --> crates/ark/src/interface.rs:397:19
    |
397 |             match &R_MAIN {
    |                   ^^^^^^^ shared reference of mutable static
    |
    = note: for more information, see issue #114447 <https://github.com/rust-lang/rust/issues/114447>
    = note: reference of mutable static is a hard error from 2024 edition
    = note: mutable statics can be written to by multiple threads: aliasing violations or data races will cause undefined behavior
    = note: `#[warn(static_mut_ref)]` on by default
help: shared references are dangerous since if there's any kind of mutation of that static while the reference lives, that's UB; use `addr_of!` instead to create a raw pointer
    |
397 |             match addr_of!(R_MAIN) {
    |                   ~~~~~~~~~~~~~~~~
```

