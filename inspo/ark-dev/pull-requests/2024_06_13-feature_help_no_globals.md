# Refactor help to remove global `PORT` and clarify logic

> <https://github.com/posit-dev/ark/pull/408>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Branched from https://github.com/posit-dev/amalthea/pull/406

When I was getting more deeply acquainted with help, I was pretty confused by `pub static mut PORT: u16 = 0;`, a global variable that gets our proxy port number (not the R help server port number). I set out to see if I could remove the global, and I think it resulted in much clearer logic for how help works!

I don't think we really need `Arc<Mutex<u16>>` because the help port is always created in the `comm_open` event now, and is immediately forwarded to both the help handler and `RMain` from there, so it seems extremely unlikely for them to be out of sync.

I tried a `CMD+R` refresh, and that does _not_ seem to open a new help comm

