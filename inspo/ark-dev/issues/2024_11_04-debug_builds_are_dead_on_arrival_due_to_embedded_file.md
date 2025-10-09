# `-debug` builds are dead on arrival due to `EmbeddedFile`

> <https://github.com/posit-dev/ark/issues/619>
> 
> * Author: @DavisVaughan
> * State: OPEN
> * Labels: 

Hadley loaded a debug build and saw:

```rust
thread 'main' panicked at crates/harp/src/lib.rs:77:29:
called `Result::unwrap()` on an `Err` value: can't open asset init.R

Stack backtrace:
   0: std::backtrace::Backtrace::create
   1: anyhow::error::<impl anyhow::Error>::msg
   2: anyhow::__private::format_err
   3: harp::modules::with_asset
   4: harp::modules::init_modules
   5: harp::initialize
   6: ark::interface::RMain::start
   7: ark::start::start_kernel
   8: ark::main
   9: core::ops::function::FnOnce::call_once
  10: std::sys::backtrace::__rust_begin_short_backtrace
  11: std::rt::lang_start::{{closure}}
  12: std::rt::lang_start_internal
  13: std::rt::lang_start
  14: _main
```

`init.R` is an embedded file, during `debug` embedded files are _read from the file system_, but he doesn't have that file!

According to https://docs.rs/rust-embed/8.5.0/rust_embed/trait.RustEmbed.html#required-methods we can use the `debug-embed` feature to force it to load from the binary even in debug mode.

This currently means all of the `-debug` releases we generate are likely DOA 

