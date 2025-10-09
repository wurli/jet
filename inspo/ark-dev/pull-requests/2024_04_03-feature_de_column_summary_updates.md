# Follow introduction of ColumnDisplayType enum in data explorer comm

> <https://github.com/posit-dev/ark/pull/293>
> 
> * Author: @wesm
> * State: MERGED
> * Labels: 

Part of https://github.com/posit-dev/positron/pull/2617, adds new top-level enum and renames things accordingly. The summary stats structs aren't used and can be disregarded here for now. 

## @wesm at 2024-04-03T01:27:13Z

@jmcphers do you know what this test failure is caused by?

```
test test_kernel ... FAILED

failures:

---- test_kernel stdout ----
thread 'test_kernel' panicked at crates/amalthea/tests/client.rs:437:13:
assertion failed: !comms.contains_key(comm_id)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    test_kernel

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.51s

error: test failed, to rerun pass `-p amalthea --test client`
```

## @wesm at 2024-04-03T16:10:34Z

Re-running the build just worked -- is that failure a known flake? If not should create an issue about it. cc also @lionel- 