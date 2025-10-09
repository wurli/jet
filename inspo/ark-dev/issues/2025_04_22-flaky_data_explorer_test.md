# Flaky data explorer test

> <https://github.com/posit-dev/ark/issues/781>
> 
> * Author: @jennybc
> * State: CLOSED
> * Labels: 

I see this intermittently locally while working on other things and running tests:

```        FAIL [  10.393s] ark::data_explorer test_live_updates
──── STDOUT:             ark::data_explorer test_live_updates

running 1 test
test test_live_updates ... FAILED

failures:

failures:
    test_live_updates

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 20 filtered out; finished in 10.36s

──── STDERR:             ark::data_explorer test_live_updates

thread 'test_live_updates' panicked at crates/ark/tests/data_explorer.rs:1081:65:
called `Result::unwrap()` on an `Err` value: Timeout
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

  Cancelling due to test failure
────────────
     Summary [  16.164s] 401 tests run: 400 passed, 1 failed, 0 skipped
        FAIL [  10.393s] ark::data_explorer test_live_updates
error: test run failed
```

The implicated line:
https://github.com/posit-dev/ark/blob/274c5a72541dba9ce9fb2cbdf616cc8e9c813029/crates/ark/tests/data_explorer.rs#L1081

## @DavisVaughan at 2025-04-22T14:16:59Z

There is the _slightest_ chance of a race condition here I think:

https://github.com/posit-dev/ark/blob/274c5a72541dba9ce9fb2cbdf616cc8e9c813029/crates/ark/src/data_explorer/r_data_explorer.rs#L234-L255

- `execution_thread()` sends `CommManagerEvent::Opened`
- This [unblocks `open_data_explorer_from_expression()`](https://github.com/posit-dev/ark/blob/274c5a72541dba9ce9fb2cbdf616cc8e9c813029/crates/ark/tests/data_explorer.rs#L1065)
- We run [R code to update x](https://github.com/posit-dev/ark/blob/274c5a72541dba9ce9fb2cbdf616cc8e9c813029/crates/ark/tests/data_explorer.rs#L1073)
- We [`EVENTS.console_prompt.emit(())` to tell the data explorer to update](https://github.com/posit-dev/ark/blob/274c5a72541dba9ce9fb2cbdf616cc8e9c813029/crates/ark/tests/data_explorer.rs#L1078C5-L1078C35)
- But there's a _slight_ chance that the data explorer isn't `listen()`ing for that event yet, since we send the `CommManagerEvent::Opened` before we call [`listen()` here](https://github.com/posit-dev/ark/blob/274c5a72541dba9ce9fb2cbdf616cc8e9c813029/crates/ark/src/data_explorer/r_data_explorer.rs#L249-L255)

The only thing I can think of to try and combat this right now is to put this block up at the top of `execution_thread()` before we send out `Opened`, to ensure we are fully prepared for console prompt events

```rust
        // Register a handler for console prompt events
        let (prompt_signal_tx, prompt_signal_rx) = unbounded::<()>();
        let listen_id = EVENTS.console_prompt.listen({
            move |_| {
                prompt_signal_tx.send(()).unwrap();
            }
        });
```