# Pass comm manager through to data viewer

> <https://github.com/posit-dev/ark/pull/67>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Branched from #66 

The point of this PR is to avoid using the `COMM_MANAGER_TX` global variable in `RDataViewer`'s `execution_thread()` where we notify the front end that the data viewer comm is opening.

This requires passing the `comm_manager_tx` object through the shell, through the REnvironment, and finally to RDataViewer. I am not sure if there is a more elegant approach but i would be open to hearing about one. I feel sort of iffy about this part.

---

I would love to get rid of the `data_viewer/globals.rs` entirely, and we are close to being able to do so now. But we have an R callable named `ps_view_data_frame()` which is just a convenience helper for viewing a data frame from the R console. To pass the comm manager through here, we need to use a global. At the very least, I was at least able to switch to the synchronous global approach in #66.

Do we think we still need `ps_view_data_frame()`? I don't feel strongly about it.

## @DavisVaughan at 2023-07-13T19:52:37Z

Maybe it is actually good to keep `ps_view_data_frame()` around? Could that be the way we hook up `View(<dataframe>)` to eventually work?

## @kevinushey at 2023-07-13T19:57:43Z

> Do we think we still need ps_view_data_frame()? I don't feel strongly about it.

I don't feel strongly either -- in the end, I think we're just going to want a way to hook up `View()` so we can do our own thing if the user tries to view an R object. That doesn't necessarily have to go through `ps_view_data_frame`.