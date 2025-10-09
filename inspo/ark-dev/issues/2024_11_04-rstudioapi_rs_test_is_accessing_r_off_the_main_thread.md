# `rstudioapi.rs` test is accessing R off the main thread

> <https://github.com/posit-dev/ark/issues/609>
> 
> * Author: @DavisVaughan
> * State: OPEN
> * Labels: 

The problem is the unguarded `harp::envvar::set_var("POSITRON_VERSION", value)` which talks to R.

This is awkward because there _is_ an `R_MAIN` but this is an integration test (unlike `data_explorer.rs` which is an integration test but without an `R_MAIN`).

So in theory we do want to use an `r_task()`, but right now `IS_TESTING` is `true` so it would just say "oh i guess i can run this right here", but really it can't. It needs to send it to the main R thread, which is probably not the thread the test is running on.

Maybe we can also look at `RMain::is_initialized()`, but I feel like this still isn't perfect:
- `R_MAIN` could be setting up, but not fully initialized yet
- We send an r-task right away
- It sees `IS_TESTING && (RMain::is_initialized() = false)` because it hasn't fully set up yet, so in that case it would still run the r-task on the current thread. Frustrating!

From https://github.com/posit-dev/ark/actions/runs/11502602809/job/32017966783:

```
running 2 tests
thread 'test_get_version' panicked at library/core/src/panicking.rs:221:5:
thread 'test_get_version' panicked at crates/ark/src/interface.rs:561:13:
panic in a function that cannot unwind
stack backtrace:
Must access `R_MAIN` on the main R thread, not thread 'test_get_version'.
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
   0:     0x55b32c02aef5 - std::backtrace_rs::backtrace::libunwind::trace::h649ab3318d3445c5
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/std/src/../../backtrace/src/backtrace/libunwind.rs:116:5
   1:     0x55b32c02aef5 - std::backtrace_rs::backtrace::trace_unsynchronized::hf4bb60c3387150c3
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/std/src/../../backtrace/src/backtrace/mod.rs:66:5
   2:     0x55b32c02aef5 - std::sys::backtrace::_print_fmt::hd9186c800e44bd00
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/std/src/sys/backtrace.rs:65:5
   3:     0x55b32c02aef5 - <std::sys::backtrace::BacktraceLock::print::DisplayBacktrace as core::fmt::Display>::fmt::h1b9dad2a88e955ff
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/std/src/sys/backtrace.rs:40:26
   4:     0x55b32c057c4b - core::fmt::rt::Argument::fmt::h351a7824f737a6a0
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/core/src/fmt/rt.rs:173:76
   5:     0x55b32c057c4b - core::fmt::write::h4b5a1270214bc4a7
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/core/src/fmt/mod.rs:1182:21
   6:     0x55b32c02773f - std::io::Write::write_fmt::hd04af345a50c312d
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/std/src/io/mod.rs:1827:15
   7:     0x55b32c02c711 - std::sys::backtrace::BacktraceLock::print::h68d41b51481bce5c
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/std/src/sys/backtrace.rs:43:9
   8:     0x55b32c02c711 - std::panicking::default_hook::{{closure}}::h96ab15e9936be7ed
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/std/src/panicking.rs:269:22
   9:     0x55b32c02c3ec - std::panicking::default_hook::h3cacb9c27561ad33
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/std/src/panicking.rs:296:9
  10:     0x55b32a7421aa - <alloc::boxed::Box<F,A> as core::ops::function::Fn<Args>>::call::hd212b1446b2b2077
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/alloc/src/boxed.rs:2084:9
  11:     0x55b32a7421aa - test::test_main::{{closure}}::hd15ff34f3f68988b
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/test/src/lib.rs:136:21
  12:     0x55b32c02cfaf - <alloc::boxed::Box<F,A> as core::ops::function::Fn<Args>>::call::hce7569f4ca5d1b64
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/alloc/src/boxed.rs:2084:9
  13:     0x55b32c02cfaf - std::panicking::rust_panic_with_hook::hfe205f6954b2c97b
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/std/src/panicking.rs:808:13
  14:     0x55b32c02cba3 - std::panicking::begin_panic_handler::{{closure}}::h6cb44b3a50f28c44
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/std/src/panicking.rs:667:13
  15:     0x55b32c02b3b9 - std::sys::backtrace::__rust_end_short_backtrace::hf1c1f2a92799bb0e
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/std/src/sys/backtrace.rs:168:18
  16:     0x55b32c02c864 - rust_begin_unwind
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/std/src/panicking.rs:[665](https://github.com/posit-dev/ark/actions/runs/11502602809/job/32017966783#step:11:666):5
  17:     0x55b32a70a855 - core::panicking::panic_nounwind_fmt::runtime::h907a0444fa61a6dc
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/core/src/panicking.rs:112:18
  18:     0x55b32a70a855 - core::panicking::panic_nounwind_fmt::h4c4dc67d0bbc166c
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/core/src/panicking.rs:122:5
  19:     0x55b32a70a8e2 - core::panicking::panic_nounwind::hb98133c151c787e4
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/core/src/panicking.rs:221:5
  20:     0x55b32a70aaa6 - core::panicking::panic_cannot_unwind::he9511e6e72319a3e
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/core/src/panicking.rs:309:5
  21:     0x55b32af8a411 - r_write_console
                               at /home/runner/work/ark/ark/crates/ark/src/interface.rs:1929:1
  22:     0x7fb25a7d5902 - REvprintf_internal
                               at /tmp/R-4.4.1/src/main/printutils.c:1052:6
  23:     0x7fb25a7d59fd - REprintf
                               at /tmp/R-4.4.1/src/main/printutils.c:898:5
  24:     0x7fb25a75b08f - Rf_check_stack_balance
                               at /tmp/R-4.4.1/src/main/eval.c:932:5
  25:     0x7fb25a75b08f - Rf_eval
                               at /tmp/R-4.4.1/src/main/eval.c:1279:6
  26:     0x7fb25a75ae50 - Rf_eval
                               at /tmp/R-4.4.1/src/main/eval.c:1226:6
  27:     0x55b32bc6b988 - libr::r::Rf_eval::h0116a2590c159505
                               at /home/runner/work/ark/ark/crates/libr/src/functions.rs:31:21
  28:     0x55b32b963364 - harp::exec::try_eval::{{closure}}::hc839c8fbf2caeaa0
                               at /home/runner/work/ark/ark/crates/harp/src/exec.rs:96:41
  29:     0x55b32b96455a - harp::exec::try_catch::callback::h8102df397fc5874e
                               at /home/runner/work/ark/ark/crates/harp/src/exec.rs:200:31
  30:     0x7fb25a72debe - R_withCallingErrorHandler
                               at /tmp/R-4.4.1/src/main/errors.c:2579:16
  31:     0x55b32bc6b636 - libr::r::R_withCallingErrorHandler::h44cd83875b2432e9
                               at /home/runner/work/ark/ark/crates/libr/src/functions.rs:31:21
  32:     0x55b32b96b01e - harp::exec::try_catch::{{closure}}::h926031ea6998fb45
                               at /home/runner/work/ark/ark/crates/harp/src/exec.rs:272:9
  33:     0x55b32b96f32a - harp::exec::top_level_exec::callback::hdf5d07c39df924ce
                               at /home/runner/work/ark/ark/crates/harp/src/exec.rs:342:31
  34:     0x7fb25a6ea8ba - R_ToplevelExec
                               at /tmp/R-4.4.1/src/main/context.c:804:2
  35:     0x55b32bc6b5d8 - libr::r::R_ToplevelExec::h733084817154de54
                               at /home/runner/work/ark/ark/crates/libr/src/functions.rs:31:21
  36:     0x55b32b96c187 - harp::exec::top_level_exec::h3036d6b85f1a5d17
                               at /home/runner/work/ark/ark/crates/harp/src/exec.rs:345:14
  37:     0x55b32b963a60 - harp::exec::try_catch::h5f1898a2d90fc101
                               at /home/runner/work/ark/ark/crates/harp/src/exec.rs:271:20
  38:     0x55b32b963113 - harp::exec::try_eval::hd6a1f8582f62f13a
                               at /home/runner/work/ark/ark/crates/harp/src/exec.rs:96:19
  39:     0x55b32b96308b - harp::exec::RFunction::call_in::h2fc69e39cccd56e4
                               at /home/runner/work/ark/ark/crates/harp/src/exec.rs:88:9
  40:     0x55b32b96301e - harp::exec::RFunction::call::hb3cafa1579aadaff
                               at /home/runner/work/ark/ark/crates/harp/src/exec.rs:83:9
  41:     0x55b32b98b196 - harp::envvar::set_var::hdc63bde33f778f68
                               at /home/runner/work/ark/ark/crates/harp/src/envvar.rs:37:5
  42:     0x55b32a70c3f8 - rstudioapi::test_get_version::h415ff5cfdf9920e9
                               at /home/runner/work/ark/ark/crates/ark/tests/rstudioapi.rs:14:5
  43:     0x55b32a70c337 - rstudioapi::test_get_version::{{closure}}::hfc49e73553401981
                               at /home/runner/work/ark/ark/crates/ark/tests/rstudioapi.rs:5:22
  44:     0x55b32a70b746 - core::ops::function::FnOnce::call_once::h21bb8dad1d799aad
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/core/src/ops/function.rs:250:5
  45:     0x55b32a7467eb - core::ops::function::FnOnce::call_once::h81f56a195fe4862e
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/core/src/ops/function.rs:250:5
  46:     0x55b32a7467eb - test::__rust_begin_short_backtrace::h919c79c8b896f9e2
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/test/src/lib.rs:624:18
  47:     0x55b32a746095 - test::run_test_in_process::{{closure}}::h7b3d5751c5b4dd75
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/test/src/lib.rs:647:60
  48:     0x55b32a746095 - <core::panic::unwind_safe::AssertUnwindSafe<F> as core::ops::function::FnOnce<()>>::call_once::hdabd61465e4dbd80
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/core/src/panic/unwind_safe.rs:272:9
  49:     0x55b32a746095 - std::panicking::try::do_call::hc813c79fd64b0a90
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/std/src/panicking.rs:557:40
  50:     0x55b32a746095 - std::panicking::try::h055c5de7e7bfc209
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/std/src/panicking.rs:521:19
  51:     0x55b32a746095 - std::panic::catch_unwind::h4265d6525195c807
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/std/src/panic.rs:350:14
  52:     0x55b32a746095 - test::run_test_in_process::he72c277a35f96567
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/test/src/lib.rs:647:27
  53:     0x55b32a746095 - test::run_test::{{closure}}::h974e632522c0fbcf
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/test/src/lib.rs:568:43
  54:     0x55b32a70e084 - test::run_test::{{closure}}::hdc2c89ce8b601dda
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/test/src/lib.rs:598:41
  55:     0x55b32a70e084 - std::sys::backtrace::__rust_begin_short_backtrace::h342cb8e53aeb2076
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/std/src/sys/backtrace.rs:152:18
  56:     0x55b32a7117b2 - std::thread::Builder::spawn_unchecked_::{{closure}}::{{closure}}::h67b1b5c1709ad95b
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/std/src/thread/mod.rs:538:17
  57:     0x55b32a7117b2 - <core::panic::unwind_safe::AssertUnwindSafe<F> as core::ops::function::FnOnce<()>>::call_once::hd8c7a030ea8b7[676](https://github.com/posit-dev/ark/actions/runs/11502602809/job/32017966783#step:11:677)
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/core/src/panic/unwind_safe.rs:272:9
  58:     0x55b32a7117b2 - std::panicking::try::do_call::h512c2ab2c15b7d31
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/std/src/panicking.rs:557:40
  59:     0x55b32a7117b2 - std::panicking::try::h5c2903f8937bc868
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/std/src/panicking.rs:521:19
  60:     0x55b32a7117b2 - std::panic::catch_unwind::h242c80217c2dbece
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/std/src/panic.rs:350:14
  61:     0x55b32a7117b2 - std::thread::Builder::spawn_unchecked_::{{closure}}::h6cb4494ebdd8caf7
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/std/src/thread/mod.rs:537:30
  62:     0x55b32a7117b2 - core::ops::function::FnOnce::call_once{{vtable.shim}}::h42193b008049ba94
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/core/src/ops/function.rs:250:5
  63:     0x55b32c0320eb - <alloc::boxed::Box<F,A> as core::ops::function::FnOnce<Args>>::call_once::ha1963004222e7822
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/alloc/src/boxed.rs:2070:9
  64:     0x55b32c0320eb - <alloc::boxed::Box<F,A> as core::ops::function::FnOnce<Args>>::call_once::h1086ced1f7c494c2
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/alloc/src/boxed.rs:2070:9
  65:     0x55b32c0320eb - std::sys::pal::unix::thread::Thread::new::thread_start::ha8af9c992ef0b208
                               at /rustc/eeb90cda1969383f56a2637cbd3037bdf598841c/library/std/src/sys/pal/unix/thread.rs:108:17
  66:     0x7fb285094ac3 - <unknown>
  67:     0x7fb28512[685](https://github.com/posit-dev/ark/actions/runs/11502602809/job/32017966783#step:11:686)0 - <unknown>
  68:                0x0 - <unknown>
thread caused non-unwinding panic. aborting.
error: test failed, to rerun pass `-p ark --test rstudioapi`
```

## @DavisVaughan at 2024-11-04T14:25:21Z

https://github.com/posit-dev/ark/pull/618 is a patch to get tests reliably passing again, but this still needs a longer term fix because it will be nice to be able to call the R API directly from integration tests