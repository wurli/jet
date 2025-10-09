# Increase test timeout from 1s -> 10s

> <https://github.com/posit-dev/ark/pull/672>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

I've been exploring nextest a bit for ark, and I would get some consistent false positive timeouts with a 1s timeout. I think having multiple processes running at once causes some kind of contention, so we need a longer timeout. With this one change (and after #669) I can run `cargo nextest run` on my mac without any issues.

I don't think having a big timeout here is that big a deal, we aren't trying to ensure its _fast_, we just dont want it to get hung.

I would really like to look into nextest as I think it will help us simplify our test infra quite a bit (no management of locks or global state or needing to worry about not cross contaminating the test R session, because each test gets its own R session). It looks like we can just switch to it if we want to with little extra effort, then start simplifying

## @DavisVaughan at 2025-01-23T21:17:50Z

Is the world just slow today? Lots of fun with timeouts, unrelated to nextest. But these also look like false alarms so I'll just up these too.

```
thread 'test_live_updates' panicked at crates/ark/tests/data_explorer.rs:1084:86:
called `Result::unwrap()` on an `Err` value: Timeout
```

## @DavisVaughan at 2025-01-23T21:21:03Z

```
thread 'lsp::completions::sources::composite::call::tests::test_completions_after_user_types_part_of_an_argument_name' panicked at core\src\panicking.rs:223:5:
panic in a function that cannot unwind
```

this seems to reproduce on the windows runner (on main too, going back a few commits) but I have no idea why. I can't reproduce on my own windows machine.

## @DavisVaughan at 2025-01-23T23:01:31Z

This one popped up again on the mac runner, but #673 should fix it

```
thread 'test_env_vars' panicked at crates/amalthea/src/fixtures/dummy_frontend.rs:195:9:
assertion failed: `Status(JupyterMessage { zmq_identities: [], header: JupyterHeader { msg_id: "24ea361b-b116-462e-a868-47d503ae01be", session: "5ccdaa85-4179-49b6-b8e2-97b484081e7e", username: "kernel", date: "2025-01-23T21:23:15.387272+00:00", msg_type: "status", version: "5.3" }, parent_header: None, content: KernelStatus { execution_state: Starting } })` does not match `Message::Welcome(data)`
```