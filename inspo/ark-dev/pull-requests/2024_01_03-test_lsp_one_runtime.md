# Move LSP's `tokio::Runtime` and `Client` to `RMain`

> <https://github.com/posit-dev/ark/pull/193>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Joint work with @lionel- 

Addresses https://github.com/posit-dev/positron/issues/1885
Addresses https://github.com/posit-dev/positron/issues/1956

- We now use 1 `tokio::Runtime` for LSP tasks. The tasks could either come from the main LSP thread or from an R callback, like `file.edit()` (which calls the LSP `show_document()` method).

- We now call `spawn()` rather than `block_on()` in `ps_editor()`. This allows R to return immediately from `ps_editor()`, preventing the deadlock that is described in https://github.com/posit-dev/positron/issues/1885#issuecomment-1874569041. We aren't worried about the fact that `file.edit()` could technically return before the file has been opened. RStudio similarly just sends off an event to open the file and then immediately returns, and it hasn't proven to be an issue there. We also weren't waiting on the file to be open before, the future we were blocking on was just confirmation that the show document request actually gets _sent_, not that it had been _received_ and acted upon.

- I've finally been able to remove all reliance on the ugly global variables we had floating around.
    - The LSP tokio `Runtime` is created in ark's `start_kernel()` and wrapped in an `Arc` so it can be sent to the LSP thread (the runtime can't be cloned). It is also sent `start_r()` to be a part of `RMain`. `Runtime` methods are all immutable, so it should not require a `Mutex`.
    - The LSP `Client` was initially trickier, but I figured out that you can _build_ the `LspService` at any time, and that is what gives us access to the `Client`, so I extracted that build call out into `build_lsp_service()` and we call that from `start_kernel()` now too. The returned service and socket are passed along to the LSP handler to actually start the LSP, but the client is instead routed to `start_r()` to be a part of `RMain`. One thing to note is that we need to be somewhat careful that the LSP actually gets initialized before sending requests through the `Client`. i.e. the LSP `initialize()` call needs to get called first. I think this is unlikely to be problematic in practice?

Proof of some things working on the windows vm that didn't work before

https://github.com/posit-dev/amalthea/assets/19150088/6ee2b771-f90c-4793-91cf-09f41441bfd0


https://github.com/posit-dev/amalthea/assets/19150088/e2dc0afb-3fcf-4477-bf62-5ec8c5c97e71




