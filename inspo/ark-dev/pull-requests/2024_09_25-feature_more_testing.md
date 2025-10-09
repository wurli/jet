# Add testing on Windows, macOS, and R 4.2

> <https://github.com/posit-dev/ark/pull/545>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

This PR has the goal of getting us the following CI structure:
- Windows (stable rust, release R)
- macOS (stable rust, release R)
- Linux (stable rust, release R)
- Linux (stable rust, 4.2 R)
- Linux (nightly rust, release R)

Adding support for macOS was incredibly simple.

Adding support for R 4.2 revealed one bug that I've fixed and called out below.

Adding support for Windows has been a nightmare. Further details below.

You should be able to trigger each OS's workflow manually through workflow dispatch, and you should also be able to turn on SSH-ing into the Linux workflow (it will pause right before running `cargo test`).

## Application manifest

I noticed that during tests on Windows, `sessionInfo()` reports that we are _not_ in a UTF-8 locale and we are on Windows Server 2012, even though the github machine is Windows Server 2022. This is a classic sign that our application manifest file wasn't being used https://github.com/posit-dev/ark/pull/178.

The manifest is being used for our main binary, `ark.exe`, but it wasn't being embedded into our test binary, like `ark-<stuff>.exe`. Making that happen was a little complicated. The crux of it is:
- `embed_resource::compile()` sets `cargo:rustc-link-arg-bins`, which doesn't target test binaries
- `embed_resource::compile_for_tests()` sets `cargo:rustc-link-arg-tests`, but that doesn't target _unit_ test binaries due to a Rust bug
- `embed_resource::compile_for_everything()` sets `cargo:rustc-link-ark`, which is the "just do it for everything" approach, and that does work. This didn't exist though, I had to ask the embed-resource maintainer for it!

So that allowed ark's unit tests to start up R in a way that R was in a UTF-8 locale and ran under Windows Server 2022.

But harp also starts R for its unit tests! And it also tests UTF-8 behavior! So I had to add a `build.rs` script to harp to also embed the manifest file there as well.

## Open comms timing issues

See https://github.com/posit-dev/ark/pull/548, I seemed to hit this error every now and then on the Windows CI and my local Windows VM

## Outstanding issues

Occasionally the test suite will hang on windows after running `test_kernel`, but I cannot figure out why. I get no information except eventually it says something like:

```
test test_kernel has been running for over 60 seconds
```

My guess is that a zmq `send()` or `recv()` is stuck, so I've set `set_sndtimeo()` and `set_rectimeo()` to try and turn these cases into a panic at test time, but I actually haven't been able to reproduce the issue since I added them, and can't reproduce this one locally.

## @DavisVaughan at 2024-09-26T21:36:49Z

There seems to be some kind of issue occasionally. I sometimes see this locally

```
[2024-09-26T21:24:52Z ERROR amalthea::kernel] While forwarding outbound message: ZeroMQ protocol error on Stdin socket: Host unreachable
test test_kernel has been running for over 60 seconds
``` 

and one of our builds above shows it too

## @lionel- at 2024-09-27T12:52:14Z

hmm that's surprising these dummy frontend sockets are on `tcp://127.0.0.1`:

https://github.com/posit-dev/ark/blob/77d94003d2d400b643a53f5f2d39fc7a9d23e714/crates/amalthea/src/fixtures/dummy_frontend.rs#L88-L95

Since you can see it locally, maybe you could set a breakpoint and explore with e.g. `telnet` whether a specific port might be at fault?