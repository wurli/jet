# Use actual Ark binary in integration tests?

> <https://github.com/posit-dev/ark/issues/562>
> 
> * Author: @lionel-
> * State: OPEN
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABwhpKcQ", name = "infra: tests", description = "", color = "bfdadc")

We currently launch Ark in a background thread but we're seeing some weirdness on Windows CI, such as stack overflows: https://github.com/posit-dev/ark/actions/runs/11143776136/job/30969594068

```
thread 'dummy_kernel' panicked at crates\ark\src\interface.rs:1653:9:
Unexpected longjump while reading console: Unexpected longjump
Likely caused by:
Error: Error: C stack usage  2069919488 is too close to the limit

...

Caused by:
  process didn't exit successfully: `D:\a\ark\ark\target\debug\deps\kernel-55e8a51febeb10a6.exe` (exit code: 0xc0000409, STATUS_STACK_BUFFER_OVERRUN)
```

It's _possible_ running R in the main thread (by running a normal Ark process) would solve this. Also it would make the integration tests a bit closer to real usage.

We're almost there since the integration tests already start a pretty normal Jupyter kernel that we connect to via zmq. But we might need to implement some process management to ensure it gets shut down after tests have run, and this management might fail in case of panic/crash.

## @lionel- at 2024-10-04T08:44:36Z

Propagation of panic info when the Ark kernel crashes: Would be done via a log file as we do in Positron. Actually kind of neat because we could enable trace logs and get more context by default. The log file would be shown in a separate section of the github action report.

## @DavisVaughan at 2024-10-05T13:27:59Z

We've had issues where we've broken something in release ark and can't catch it during testing because testing behavior works slightly differently https://github.com/posit-dev/ark/pull/566#issuecomment-2394939383 - in theory this would help avoid that