# Teach the comm manager how to handle info requests

> <https://github.com/posit-dev/ark/pull/548>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

In the past I have occasionally seen this on our Linux CI, and it seems to occur quite often on our Windows CI

```
---- test_kernel stdout ----
thread 'test_kernel' panicked at crates\amalthea\tests\client.rs:437:13:
assertion failed: !comms.contains_key(comm_id)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    test_kernel

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.58s

error: test failed, to rerun pass `-p amalthea --test client`
```

If we go look at that test, it looks like this:

```rust
    // Test closing the comm we just opened
    info!("Sending comm close request to the kernel");
    frontend.send_shell(CommClose {
        comm_id: comm_id.to_string(),
    });

    // Absorb the IOPub messages that the kernel sends back during the
    // processing of the above `CommClose` request
    info!("Receiving comm close IOPub messages from the kernel");
    frontend.recv_iopub(); // Busy
    frontend.recv_iopub(); // Idle

    // Test to see if the comm is still in the list of comms after closing it
    // (it should not be)
    info!("Requesting comm info from the kernel (to test closing)");
    frontend.send_shell(CommInfoRequest {
        target_name: "variables".to_string(),
    });
    let reply = frontend.recv_shell();
    match reply {
        Message::CommInfoReply(request) => {
            info!("Got comm info: {:?}", request);
            // Ensure the comm we just closed not present in the list of comms
            let comms = request.content.comms;
            assert!(!comms.contains_key(comm_id));
        },
        _ => {
            panic!(
                "Unexpected message received (expected comm info): {:?}",
                reply
            );
        },
    }
```

Here's what was happening:
- Both `Shell` and the `CommManager` maintain a list of `open_comms` that _should_ be mirror images of one another
- `Shell` gets `CommClose` and forwards that to `CommManager`
- In theory, `CommManager`:
    - Processes that event
    - Drops the comm from its list of `open_comms`
    - Sends `Shell` back a `CommShellEvent::Close` so `Shell` can also drop that comm from its `open_comms`
-  `Shell` gets `CommInfoRequest` and does _not_ forward that to `CommManager`. It uses its internal list of `open_comms` to reply to this request with.

The problem is that sometimes the `CommManager` sends back `CommShellEvent::Close` too slowly, and `Shell` gets that `CommInfoRequest` before its been told by the comm manager that the comm has actually closed. So the `open_comms` of `Shell` are temporarily out of sync with the `open_comms` of `CommManager`, and the test fails because `Shell`'s list of `open_comms` _do_ still contain that key.

I've reworked this by allowing `CommManager` to accept one off _requests_. In this case, a special `Info` request that comes with its own `tx` that it sends the reply back over. Our `Shell` no longer manages `open_comms`, and instead makes an `Info` request whenever it needs to report on the list of open comms - making `CommManager` the "source of truth" for the open comms. I believe this should solve the issue.

