# `ui_comm` test failure

> <https://github.com/posit-dev/ark/issues/927>
> 
> * Author: @jennybc
> * State: CLOSED
> * Labels: 

I'm seeing this test fail consistently for me locally on Windows. I don't know why it's not showing up in CI:

```
failures:

---- ui::ui::tests::test_ui_comm stdout ----

thread 'ui::ui::tests::test_ui_comm' (16212) panicked at crates\ark\src\ui\ui.rs:299:14:
called `Result::unwrap()` on an `Err` value: Timeout
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

https://github.com/posit-dev/ark/blob/ad2dd67f3eb9ad292c98aa7e4e614732a7ab81ce/crates/ark/src/ui/ui.rs#L293-L299

## @jennybc at 2025-09-24T22:49:49Z

Hmmm, this might be because I'm doing `cargo test` 🤔. I'll do the nextest and just setup described in BUILDING.md and see if that changes anything.

## @jennybc at 2025-09-24T23:43:07Z

Yes, now that I'm doing everything as laid out in BUILDING.md, this test has failure has gone away. Closing.