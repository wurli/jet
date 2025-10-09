# Use `static LOGGER` not `static mut LOGGER`

> <https://github.com/posit-dev/ark/pull/215>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

When updating rust to a more recent version, I see this warning:

```
warning: shared reference of mutable static is discouraged
   --> crates/ark/src/logger.rs:191:29
    |
191 |             log::set_logger(&LOGGER).unwrap();
    |                             ^^^^^^^ shared reference of mutable static
    |
    = note: for more information, see issue #114447 <https://github.com/rust-lang/rust/issues/114447>
    = note: reference of mutable static is a hard error from 2024 edition
    = note: mutable statics can be written to by multiple threads: aliasing violations or data races will cause undefined behavior
help: shared references are dangerous since if there's any kind of mutation of that static while the reference lives, that's UB; use `addr_of!` instead to create a raw pointer
    |
191 |             log::set_logger(addr_of!(LOGGER)).unwrap();
    |                             ~~~~~~~~~~~~~~~~
```

We create a `static mut LOGGER` with some initial values, and then replace those values 1 time on initialization before handing it off to `log::set_logger()`, which requires a `&'static Logger` _static_ reference.

Handing out references to a _mutable_ static like this is discouraged and will eventually be an error.

I've worked around this by using the `Logger` + `LoggerInner` pattern, where the inner bits are guarded by a `Mutex`, giving us:
- Interior mutability, allowing us to set to `None` at compile time and initialize later
- A `static LOGGER`, because the interior mutability doesn't require `LOGGER` itself to be `mut`

The most annoying part about this is that the `level` also moves into the `Mutex`, making `enabled()` checks a little more expensive, but I think that is ok for our uses?

