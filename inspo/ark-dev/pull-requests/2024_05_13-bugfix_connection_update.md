# Handle connection updates and the `hint` argument

> <https://github.com/posit-dev/ark/pull/352>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/3120

This PR fixes two issues:

- We need `hint` named argument in `connectionUpdated`, this argument seems to be ignored by RStudio's connection contract but unfortunatelly some R implementations provide it as a `NULL`. See [here](https://github.com/rstudio/rstudio/blob/6af5c0d231bd6fb2e50dcd980be49ecc2bf64c16/src/gwt/src/org/rstudio/studio/client/workbench/views/connections/ui/ObjectBrowser.java#L64) for RStudio's usage of hint.

- We need to handle connection updates, which we were just completely ignoring. With this PR, we send a front-ent event that request a connections pane refresh.

