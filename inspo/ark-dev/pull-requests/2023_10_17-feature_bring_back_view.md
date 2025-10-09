# Bring back `.ps.view_data_frame()`

> <https://github.com/posit-dev/ark/pull/117>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

A step towards `View()` support. This will likely be our Rust entry point, I'm just not quite sure how we want to mask `View()` on the R side. But hopefully this is enough to unblock @jgutman on that front.

In https://github.com/posit-dev/amalthea/pull/67/commits/a5b0c7cc5314968c9497032ef43150ce36fba3a4 we removed this function because it relied on global state that we were having difficulty managing. Since then we have centralized that state into `RMain` (thanks @lionel-!) so it is now straightforward to add the `comm_manager_tx` to `RMain` and then access it from `.ps.view_data_frame()`.

