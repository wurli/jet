# Use roxygen-like tag to export positron functions instead of private/public folder split

> <https://github.com/posit-dev/ark/pull/206>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

With this change we now have a single folder the R sources of positron instead of a private/public split. To export a function to the attached environment `tools:positron`, include an `#' @export` tag above the object. This relies on the functionality introduced in https://github.com/posit-dev/amalthea/pull/195.

This means we can now have internal utils implemented in the same file as exported functions, and we won't mistakenly export internal utils and pollute the search path (we had 3 functions inadvertently exported before this PR).

The modules folder has separate `positron` and `rstudio` folders so that the export tags may target the correct environment, `tools:rstudio` in the latter case. This rstudio folder was previously called `rstudioapi`. I renamed it because it aims to populate the `tools:rstudio` environment and mimick RStudio functionality.

@kevinushey I didn't update the module watcher functionality because we now have easy reloading of ark without having to reload the whole positron window. Do you think that is sufficient? If you think that it'd still be helpful to watch the folder, I'm happy to update the functionality to the new layout.

## @DavisVaughan at 2024-01-17T14:09:11Z

Noting that this will also need to be updated on the positron side
https://github.com/posit-dev/positron/blob/f271d3552a6ba9da1e1a648e7081aabe33e57159/extensions/positron-r/positron.json#L21-L30

## @kevinushey at 2024-01-17T21:10:27Z

@lionel- the module watcher was added so that you could write and debug Ark's internal R scripts while working on Positron itself; will that still work with these changes?

## @lionel- at 2024-01-17T22:40:21Z

The way I've been working is to restart ark from positron, which automatically updates the modules. I figured the watcher might have been useful prior to the ability to restart ark but could possibly now be superseded. Is this the case?

## @lionel- at 2024-01-26T15:32:51Z

Since there is a hesitation I've reintroduced the watcher.