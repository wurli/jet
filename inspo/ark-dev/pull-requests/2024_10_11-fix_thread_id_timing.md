# Set `R_MAIN_THREAD_ID` at the very beginning of setup again

> <https://github.com/posit-dev/ark/pull/580>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/4973

Also creates a new frontend variant called `DummyArkFrontendRprofile` - a `Console` ark that supports loading `.Rprofile`. This allows us to test that we can load `.Rprofile`s correctly on startup. Notably you can only call `DummyArkFrontendRprofile::lock()` once per process, because you can only load an `.Rprofile` once! This is enforced with a panic if you call `lock()` twice in a single integration test.

## @DavisVaughan at 2024-10-11T01:05:31Z

I'm mildly surprised that the test passes on Windows, I thought the Rprofile would actually run twice due to https://github.com/posit-dev/positron/issues/4253, causing multiple IOPub messages

## @jennybc at 2024-10-11T02:26:46Z

FWIW this gets me back to a happy place in a positron dev build 🎉
