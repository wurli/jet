# Jupyter compatible plots and updatable plots

> <https://github.com/posit-dev/ark/pull/73>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/rstudio/positron/issues/963
Addresses https://github.com/rstudio/positron/issues/459
Requires https://github.com/rstudio/positron/pull/987

- Plots are now Jupyter compatible. We now send a `DisplayData` message over IOPub if we are _not_ in Positron (detected through the new `positron_connected()` function).
- We no longer open a `positron.plot` comm if we detect that we are not in Positron
- Plots can now be _updated_ (with https://github.com/rstudio/positron/pull/987).
    - In Positron, we now send an `Update` event to Positron over the `positron.plot` comm which triggers Positron to send back an RPC request once it is ready to receive the updated version of the plot. The RPC request is exactly the same render request that we were already using, so that's nice.
    - In not-Positron, we send an `UpdateDisplayData` message (linked to a `DisplayData` cell by a `display_id`, which is the same as the plot page `id`) with the new plot.

The _update_ behavior was a little tricky. Previously we only ever told Positron "hey we have a plot for you" during `new_page()` events. This didn't work with plot updates because a new page isn't generated at all! Instead, we now track two things:
- Whether or not we are on a new page, through `_new_page: bool` and updated in the `new_page()` hook
- Whether or not there are any changes to render (regardless of new page-ness), through `_changes: bool` and updated in the `mode()` hook whenever the mode is 1

After the R code execution has finished (but before we go back to `idle`), we do a quick check to see if `_changes` is `true`. If so, then:
- If `_new_page` is also true, we have a new plot to make
- Otherwise, we are updating an existing plot based on the current plot page `_id`

At that point we send Positron the message of "hey we have a plot for you" or "hey we have an update for you", and then we check in the frequently-called `on_process_events()` for Positron to respond with an RPC request for the actual plot, which we finally render and send back.


https://github.com/posit-dev/amalthea/assets/19150088/a36912c2-1cc1-45a5-886e-9fa28787a279



https://github.com/posit-dev/amalthea/assets/19150088/cd86e75a-1cfc-4ec4-81c5-1c37aee2d1d6



---

Post-alpha thoughts:
- Plot dimensions in Jupyter are currently hardcoded, but we should probably make them dynamic based on an R global option, like `ark.plot.height` or something.

## @DavisVaughan at 2023-08-09T20:34:33Z

> To confirm, do you we also draw a new plot for e.g. grid.newpage()?

Yea this is what ggplot2 uses and I did somewhat extensive testing with that too.

`grid.newpage()` calls its own C `L_newpage()` function, and that correctly calls `GENewPage()` which invokes our hook
https://github.com/wch/r-source/blob/6b5d4ca5d1e3b4b9e4bbfb8f75577aff396a378a/src/library/grid/src/grid.c#L1260-L1296
https://github.com/wch/r-source/blob/6b5d4ca5d1e3b4b9e4bbfb8f75577aff396a378a/src/main/engine.c#L2855