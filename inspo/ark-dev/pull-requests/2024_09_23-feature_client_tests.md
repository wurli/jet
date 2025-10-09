# Draft integration tests for Ark

> <https://github.com/posit-dev/ark/pull/542>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

- Export Amalthea client harness as `DummyFrontend`.
- Export `start_r` so it can be used in unit tests. Moved from `main.rs` to `start.rs` (I took the opportunity to move things around in `main.rs` for clarity but no changes to the code).
- Draft kernel protocol tests in `ark/tests/kernel.rs`.

TODO: Flesh out tests so that we can be confident we don't break anything in #536 

## @lionel- at 2024-09-25T11:33:58Z

- The dummy kernel is now a singleton object. Call the `lock()` method to initialize it if needed, and lock access to the singleton in a test thread.

  When the lock guard goes out of scope, it checks that all sockets have no incoming data. This way we don't forget to assert replies from the backend and we make sure the tests are (mostly) independent, which is important since they share the same R and kernel instances.

- `DummyFrontend` in Amalthea gains many more methods to receive and assert specific messages. The goal is to allow very terse tests that are easy to write.

- I've moved `start_r()` to `RMain::start()`.

- New `RMain::wait_r_initialized()` method. This is a thread-safe method that resolves when R has finished starting up. Used in the dummy kernel.

- `RMain::initialized()` has been reworked so it's thread-safe. That was not necessary here but it seems like a good principle for all static methods to be callable from another thread.

- We now detect `R_HOME` from the `PATH` via `R RHOME`. This helps with unit tests. Following this change we no longer hard-code `R_HOME` into installed kernel specs, which allows users to control which version of R to run by manipulating their `PATH`. While I was in there I also set `RUST_LOG` to `"error"` to address https://github.com/posit-dev/positron/issues/2098.
 
- We now implement the `Suicide` frontend method so that we get an error message an a backtrace when R fails to start up. (Done as part of debugging CI failures)

- @DavisVaughan determined yesterday that CI was failing because `cfg()` doesn't run in integration tests (tests in `crate/tests` as opposed to `crate/src`). To fix this, we now use a global variable aliased to the presence of a `testing` feature flag in the harp crate to decide whether to set unlimited stack space (needed in unit tests since they access R from different threads).

   Here is the motivation for this unusual setup:
  
   - Unfortunately we can't use `cfg(test)` in integration tests because they
     are treated as an external crate.
  
   - Unfortunately we cannot move some of our integration tests to `src/`
     because they must be run in their own process (e.g. because they are
     running R).
  
   - Unfortunately we can't use the workaround described in
     https://github.com/rust-lang/cargo/issues/2911#issuecomment-749580481
     to enable a test-only feature in a self dependency in the dev-deps section
     of the manifest file because Rust-Analyzer doesn't support such
     circular dependencies: https://github.com/rust-lang/rust-analyzer/issues/14167.
     So instead we use the same trick with harp rather than ark, so that there
     is no circular dependency, which fixes the issue with Rust-Analyzer.
  
   - Unfortunately we can't query the features enabled in a dependency with `cfg`.
     So instead we define a global variable here that can then be checked at
     runtime in Ark.

## @lionel- at 2024-09-25T12:41:34Z

It's probably worth merging in this state because I'm encountering protocol issues when trying to make a test for errors.

## @DavisVaughan at 2024-09-25T12:48:07Z

Would you mind using the new testing global to get rid of the `true` argument of `modules::initialize(true)`? I think that would be useful / more consistent if that is possible

## @lionel- at 2024-09-25T13:01:55Z

Good idea, done!