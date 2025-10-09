# Refresh diagnostics on open, on close, on change, and after R execution

> <https://github.com/posit-dev/ark/pull/224>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/2155
Part of https://github.com/posit-dev/positron/issues/1005

Okay, after much fiddling, I've come up with the following Alpha level scheme for our diagnostics:
- `did_open()`: Runs diagnostics for that file
- `did_change()`: Runs diagnostics for that file if we are up to date on changes
- `did_close()`: Clears diagnostics for that file
- did R console execution: Runs diagnostics for all open files

There is much more we can eventually do, like running diagnostics for the whole workspace, maintaining diagnostics on close (and updating them through a "related documents" feature), etc, but this should be a pretty nice improvement.

I'd also like to make a follow up PR or two that trims down our usage of R while diagnostics are being run. We currently look up the set of installed packages and environment symbols at each diagnostic call, but really I think we can cache these and only update them when we do a diagnostic refresh after R console execution (and on startup).

A few of the high level changes:
- Replaced `lsp_client` with the overarching `lsp_backend` in `RMain`, because the backend holds the list of the currently open `documents`, and we need that to refresh them after an R console execution
- You'll see an `IndexerStateManager`, this is solely used to ensure that we don't run any diagnostics until the indexer has had a chance to run (30 second timeout). The indexer is a very important part of our diagnostics, and we can return some pretty bad ones if we try to run diagnostics before the indexer has finished. Since the indexer runs in a tokio thread, this is totally possible and does happen for me in dplyr where it takes around 3 second for the indexer to fully run. I think I've managed to make the initialization check pretty cheap.
- After `handle_execute_request()` (R execution), we refresh diagnostics in all open documents by requesting an `r_async_task()` that loops over the open documents and spawns a task on the LSP tokio runtime for each of them, requesting a diagnostic refresh.

Note: We are still undecided on https://github.com/posit-dev/positron/issues/1325, which is about what we should do when you are working on an R package and haven't run `load_all()` yet, or if you have functions in a script with a `library(dplyr)` call at the top but you haven't run it yet. But the big benefit of this PR is that when you _do_ run them, the diagnostics automatically update.

https://github.com/posit-dev/amalthea/assets/19150088/ebd0f7d8-d4c2-4e77-b1d1-9709859b8f53

