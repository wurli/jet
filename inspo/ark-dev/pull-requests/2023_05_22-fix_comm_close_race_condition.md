# Emit busy/idle states from `handle_comm_close()`

> <https://github.com/posit-dev/ark/pull/6>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

This PR is very very similar to https://github.com/rstudio/positron/pull/358

This PR was motivated by CI failing (occasionally) in 
https://github.com/rstudio/positron/pull/606

```
running 1 test
test test_kernel ... FAILED

failures:

---- test_kernel stdout ----
thread '<unnamed>' panicked at 'called `Result::unwrap()` on an `Err` value: RecvError', crates/amalthea/tests/shell/mod.rs:237:43
error: test failed, to rerun pass `-p amalthea --test client`
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
thread 'test_kernel' panicked at 'assertion failed: !comms.contains_key(comm_id)', crates/amalthea/tests/client.rs:406:13


failures:
    test_kernel

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.52s

error Command failed with exit code 101.
```

<details>

Full backtrace

Abbreviated backtrace:

```
running 1 test
test test_kernel ... FAILED

failures:

---- test_kernel stdout ----
thread '<unnamed>' panicked at 'called `Result::unwrap()` on an `Err` value: RecvError', crates/amalthea/tests/shell/mod.rs:237:43
stack backtrace:
   0: rust_begin_unwind
             at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/panicking.rs:575:5
   1: core::panicking::panic_fmt
             at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/core/src/panicking.rs:65:14
   2: core::result::unwrap_failed
             at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/core/src/result.rs:1791:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.
thread 'test_kernel' panicked at 'assertion failed: !comms.contains_key(comm_id)', crates/amalthea/tests/client.rs:406:13
stack backtrace:
   0: rust_begin_unwind
             at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/panicking.rs:575:5
   1: core::panicking::panic_fmt
             at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/core/src/panicking.rs:65:14
   2: core::panicking::panic
             at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/core/src/panicking.rs:115:5
   3: client::test_kernel
   4: core::ops::function::FnOnce::call_once
   5: core::ops::function::FnOnce::call_once
             at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/core/src/ops/function.rs:251:5
note: Some details are omitted, run with `RUST_BACKTRACE=full` for a verbose backtrace.


failures:
    test_kernel

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.55s
```

Full backtrace 

```
running 1 test
test test_kernel ... FAILED

failures:

---- test_kernel stdout ----
thread '<unnamed>' panicked at 'called `Result::unwrap()` on an `Err` value: RecvError', crates/amalthea/tests/shell/mod.rs:237:43
stack backtrace:
   0:        0x1055071c2 - std::backtrace_rs::backtrace::libunwind::trace::h74d17ea919046bae
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/../../backtrace/src/backtrace/libunwind.rs:93:5
   1:        0x1055071c2 - std::backtrace_rs::backtrace::trace_unsynchronized::h2fc77fd5a14165ac
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/../../backtrace/src/backtrace/mod.rs:66:5
   2:        0x1055071c2 - std::sys_common::backtrace::_print_fmt::h2687aa7717781133
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/sys_common/backtrace.rs:65:5
   3:        0x1055071c2 - <std::sys_common::backtrace::_print::DisplayBacktrace as core::fmt::Display>::fmt::hdc69a6f447628e71
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/sys_common/backtrace.rs:44:22
   4:        0x105523dca - core::fmt::write::hb9e764fa47ae8444
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/core/src/fmt/mod.rs:1209:17
   5:        0x10550396c - std::io::Write::write_fmt::h55cbab6d9bd3b1cc
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/io/mod.rs:1682:15
   6:        0x105506f8a - std::sys_common::backtrace::_print::h882e8250b822b8b0
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/sys_common/backtrace.rs:47:5
   7:        0x105506f8a - std::sys_common::backtrace::print::h488fe4c0b1fb9d50
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/sys_common/backtrace.rs:34:9
   8:        0x105508dc6 - std::panicking::default_hook::{{closure}}::h5618ea3156b8b833
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/panicking.rs:267:22
   9:        0x105508aa6 - std::panicking::default_hook::h0421c26a8a92801c
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/panicking.rs:283:9
  10:        0x1053dcc29 - <alloc::boxed::Box<F,A> as core::ops::function::Fn<Args>>::call::h30410c450cfcec6d
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/alloc/src/boxed.rs:2001:9
  11:        0x1053dcc29 - test::test_main::{{closure}}::h97f5cfefb46a837f
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/test/src/lib.rs:135:21
  12:        0x105509531 - <alloc::boxed::Box<F,A> as core::ops::function::Fn<Args>>::call::h43102c9332c70021
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/alloc/src/boxed.rs:2001:9
  13:        0x105509531 - std::panicking::rust_panic_with_hook::h57383cd32463c250
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/panicking.rs:692:13
  14:        0x1055092c3 - std::panicking::begin_panic_handler::{{closure}}::h1d1f7305cfe67fdd
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/panicking.rs:579:13
  15:        0x105507658 - std::sys_common::backtrace::__rust_end_short_backtrace::hd8e12e82ff026bae
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/sys_common/backtrace.rs:137:18
  16:        0x105508f8d - rust_begin_unwind
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/panicking.rs:575:5
  17:        0x105535003 - core::panicking::panic_fmt::h7894cd1015cfee41
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/core/src/panicking.rs:65:14
  18:        0x1055352c5 - core::result::unwrap_failed::h3077b600131e58d4
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/core/src/result.rs:1791:5
  19:        0x10530622d - std::sys_common::backtrace::__rust_begin_short_backtrace::hd023b3c5cd7a9b40
  20:        0x105318eb8 - core::ops::function::FnOnce::call_once{{vtable.shim}}::h176bb329eec66b39
  21:        0x10550d7b7 - <alloc::boxed::Box<F,A> as core::ops::function::FnOnce<Args>>::call_once::h2611f89e824929e3
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/alloc/src/boxed.rs:1987:9
  22:        0x10550d7b7 - <alloc::boxed::Box<F,A> as core::ops::function::FnOnce<Args>>::call_once::hb87e0c9c6cb0305b
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/alloc/src/boxed.rs:1987:9
  23:        0x10550d7b7 - std::sys::unix::thread::Thread::new::thread_start::h7b576c3bd89f934a
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/sys/unix/thread.rs:108:17
  24:     0x7ff8150834e1 - __pthread_start
thread 'test_kernel' panicked at 'assertion failed: !comms.contains_key(comm_id)', crates/amalthea/tests/client.rs:406:13
stack backtrace:
   0:        0x1055071c2 - std::backtrace_rs::backtrace::libunwind::trace::h74d17ea919046bae
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/../../backtrace/src/backtrace/libunwind.rs:93:5
   1:        0x1055071c2 - std::backtrace_rs::backtrace::trace_unsynchronized::h2fc77fd5a14165ac
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/../../backtrace/src/backtrace/mod.rs:66:5
   2:        0x1055071c2 - std::sys_common::backtrace::_print_fmt::h2687aa7717781133
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/sys_common/backtrace.rs:65:5
   3:        0x1055071c2 - <std::sys_common::backtrace::_print::DisplayBacktrace as core::fmt::Display>::fmt::hdc69a6f447628e71
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/sys_common/backtrace.rs:44:22
   4:        0x105523dca - core::fmt::write::hb9e764fa47ae8444
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/core/src/fmt/mod.rs:1209:17
   5:        0x10550396c - std::io::Write::write_fmt::h55cbab6d9bd3b1cc
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/io/mod.rs:1682:15
   6:        0x105506f8a - std::sys_common::backtrace::_print::h882e8250b822b8b0
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/sys_common/backtrace.rs:47:5
   7:        0x105506f8a - std::sys_common::backtrace::print::h488fe4c0b1fb9d50
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/sys_common/backtrace.rs:34:9
   8:        0x105508dc6 - std::panicking::default_hook::{{closure}}::h5618ea3156b8b833
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/panicking.rs:267:22
   9:        0x105508aa6 - std::panicking::default_hook::h0421c26a8a92801c
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/panicking.rs:283:9
  10:        0x1053dcc29 - <alloc::boxed::Box<F,A> as core::ops::function::Fn<Args>>::call::h30410c450cfcec6d
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/alloc/src/boxed.rs:2001:9
  11:        0x1053dcc29 - test::test_main::{{closure}}::h97f5cfefb46a837f
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/test/src/lib.rs:135:21
  12:        0x105509531 - <alloc::boxed::Box<F,A> as core::ops::function::Fn<Args>>::call::h43102c9332c70021
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/alloc/src/boxed.rs:2001:9
  13:        0x105509531 - std::panicking::rust_panic_with_hook::h57383cd32463c250
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/panicking.rs:692:13
  14:        0x105509282 - std::panicking::begin_panic_handler::{{closure}}::h1d1f7305cfe67fdd
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/panicking.rs:577:13
  15:        0x105507658 - std::sys_common::backtrace::__rust_end_short_backtrace::hd8e12e82ff026bae
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/sys_common/backtrace.rs:137:18
  16:        0x105508f8d - rust_begin_unwind
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/panicking.rs:575:5
  17:        0x105535003 - core::panicking::panic_fmt::h7894cd1015cfee41
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/core/src/panicking.rs:65:14
  18:        0x1055350d7 - core::panicking::panic::he7f1697d1ff9d4f7
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/core/src/panicking.rs:115:5
  19:        0x1052f66fb - client::test_kernel::h374543348bad04c1
  20:        0x1052efc0e - core::ops::function::FnOnce::call_once::hb139c51546ed5afb
  21:        0x1053e5582 - core::ops::function::FnOnce::call_once::h1b37aa53eb0288c2
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/core/src/ops/function.rs:251:5
  22:        0x1053e5582 - test::__rust_begin_short_backtrace::h91dc8defba47b824
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/test/src/lib.rs:599:18
  23:        0x1053b63f1 - test::run_test::{{closure}}::h59fe59b3e2e9c134
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/test/src/lib.rs:590:30
  24:        0x1053b63f1 - core::ops::function::FnOnce::call_once{{vtable.shim}}::hfe07c7668eb7cef4
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/core/src/ops/function.rs:251:5
  25:        0x1053e42b5 - <alloc::boxed::Box<F,A> as core::ops::function::FnOnce<Args>>::call_once::h119e8c1162dcbe11
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/alloc/src/boxed.rs:1987:9
  26:        0x1053e42b5 - <core::panic::unwind_safe::AssertUnwindSafe<F> as core::ops::function::FnOnce<()>>::call_once::h1e21bee716778d6a
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/core/src/panic/unwind_safe.rs:271:9
  27:        0x1053e42b5 - std::panicking::try::do_call::h505c645f6cc32320
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/panicking.rs:483:40
  28:        0x1053e42b5 - std::panicking::try::hcd4ccdd51a99574c
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/panicking.rs:447:19
  29:        0x1053e42b5 - std::panic::catch_unwind::he644ea552303b474
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/panic.rs:137:14
  30:        0x1053e42b5 - test::run_test_in_process::h474e3e304fb437ec
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/test/src/lib.rs:622:27
  31:        0x1053e42b5 - test::run_test::run_test_inner::{{closure}}::he043dad562eb79eb
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/test/src/lib.rs:516:39
  32:        0x1053b02a0 - test::run_test::run_test_inner::{{closure}}::hf330fd255ef63160
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/test/src/lib.rs:543:37
  33:        0x1053b02a0 - std::sys_common::backtrace::__rust_begin_short_backtrace::h129fabb09c3ec406
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/sys_common/backtrace.rs:121:18
  34:        0x1053b603c - std::thread::Builder::spawn_unchecked_::{{closure}}::{{closure}}::h1e1ef254b91ed487
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/thread/mod.rs:551:17
  35:        0x1053b603c - <core::panic::unwind_safe::AssertUnwindSafe<F> as core::ops::function::FnOnce<()>>::call_once::h7d22885f3d408005
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/core/src/panic/unwind_safe.rs:271:9
  36:        0x1053b603c - std::panicking::try::do_call::h17eed10251f2fb85
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/panicking.rs:483:40
  37:        0x1053b603c - std::panicking::try::h1d7b2274374038bc
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/panicking.rs:447:19
  38:        0x1053b603c - std::panic::catch_unwind::hb1c659c5dd663245
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/panic.rs:137:14
  39:        0x1053b603c - std::thread::Builder::spawn_unchecked_::{{closure}}::h0a23b620e97f84a3
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/thread/mod.rs:550:30
  40:        0x1053b603c - core::ops::function::FnOnce::call_once{{vtable.shim}}::h659d68ce1aa86c29
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/core/src/ops/function.rs:251:5
  41:        0x10550d7b7 - <alloc::boxed::Box<F,A> as core::ops::function::FnOnce<Args>>::call_once::h2611f89e824929e3
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/alloc/src/boxed.rs:1987:9
  42:        0x10550d7b7 - <alloc::boxed::Box<F,A> as core::ops::function::FnOnce<Args>>::call_once::hb87e0c9c6cb0305b
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/alloc/src/boxed.rs:1987:9
  43:        0x10550d7b7 - std::sys::unix::thread::Thread::new::thread_start::h7b576c3bd89f934a
                               at /rustc/69f9c33d71c871fc16ac445211281c6e7a340943/library/std/src/sys/unix/thread.rs:108:17
  44:     0x7ff8150834e1 - __pthread_start


failures:
    test_kernel

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.54s
```

</details>

I can reproduce this locally by running `cargo test --release` a few times locally. Note that I really did need `--release` to reproduce. I _think_ this is a race condition, and my guess is that the debug build runs rust code slower than the release build? So possibly there is just enough time to avoid the race condition in the debug build?

After implementing this fix, I can no longer trigger the error.

---

As noted in https://github.com/rstudio/positron/pull/358, we really are supposed to send busy/idle states on the IOPub channel when processing a shell message. We do this for `handle_request()` and `handle_comm_open()` (as of https://github.com/rstudio/positron/pull/358), but we don't do this for `handle_comm_close()`.

Here is the jupyter doc section that tells us we should be sending these busy/idle states:
https://jupyter-client.readthedocs.io/en/stable/messaging.html#request-reply

With this fix, we now always send busy/idle states for all cases covered by:
https://github.com/posit-dev/amalthea/blob/13a9dd9903b9b4c12f1c596626de6fe64430478b/crates/amalthea/src/socket/shell.rs#L131-L156

## @DavisVaughan at 2023-05-22T19:35:00Z

@jmcphers since you implemented that other PR, seems like you'd be best to ask for a review here.