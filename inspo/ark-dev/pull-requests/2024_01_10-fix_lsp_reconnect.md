# Rebuild LSP service, socket, and client on reconnect

> <https://github.com/posit-dev/ark/pull/201>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Follow up to #193 

I accidentally introduced a bug related to UI refreshes with `CMD + R`. If you run that with an active R session, then you should get a panic related to the LSP (something about unwrapping a `None`).

---

It turns out that we can't move the LSP service, socket, and client to `start_kernel()`. That code will get run exactly 1 time per ark session lifetime, but the `lsp_start()` method will get called every time the UI is refreshed (i.e. `CMD + R`) (which prompts a new `comm_open` request). This means we really do need "fresh" instances of these objects after each refresh.

Most of the code from #193 related to these objects is now reverted, moving their creation back into `start_lsp()`.

However, we now also use an `r_task()` to ship the `Client` over to `RMain` after each refresh.

`R_MAIN` actually _should_ be started up by the time we get here to call this `r_task()`, because the `Lsp::start()` method that calls `start_lsp()` will wait for the `kernel_init_tx` notification that is sent out by the first `r_read_console()` iteration, and `R_MAIN` exists at that point. I'm convinced that even if it didn't exist yet, then `r_task()` would correctly block until the channel for communicating with the main R thread was set up, due to `get_tasks_tx()`.

I did some testing and things like `usethis::edit_r_profile()` (which use the `Client` stored in `R_MAIN`) work as expected before and after a `CMD + R` refresh, which makes me somewhat confident that the client is getting refreshed properly.

