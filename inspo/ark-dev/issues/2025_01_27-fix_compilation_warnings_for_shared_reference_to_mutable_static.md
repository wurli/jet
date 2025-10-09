# Fix compilation warnings for shared reference to mutable static

> <https://github.com/posit-dev/ark/issues/661>
> 
> * Author: @lionel-
> * State: CLOSED
> * Labels: 

With recent versions of stable Rust, we now get a bunch of compilation warnings like:

```
warning: creating a shared reference to mutable static is discouraged
    --> crates/ark/src/interface.rs:1953:31
     |
1953 |             unsafe { Rf_error(ERROR_BUF.as_ref().unwrap().as_ptr()) };
     |                               ^^^^^^^^^^^^^^^^^^ shared reference to mutable static
     |
     = note: for more information, see <https://doc.rust-lang.org/nightly/edition-guide/rust-2024/static-mut-references.html>
     = note: shared references to mutable statics are dangerous; it's undefined behavior if the static is mutated or if a mutable reference is created for it while the shared reference lives
```

