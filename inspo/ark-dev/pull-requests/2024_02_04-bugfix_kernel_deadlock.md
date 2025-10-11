# Send UI status from async task to avoid kernel deadlock

> <https://github.com/posit-dev/ark/pull/236>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

Extracted from #235, see discussion in #234.

I figured out a way to prevent the instability of LSP reloading on the frontend side. So this PR fixes the kernel deadlock without delaying the kernel heartbeat, which could lead to other problems if Rprofile is taking a long time (e.g. tries to reach the Internet with a slow connection or makes updates on startup).

## @DavisVaughan at 2024-02-12T20:37:10Z

Should be able to close https://github.com/posit-dev/amalthea/pull/235 after this merges
