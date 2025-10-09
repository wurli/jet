# Add RPCs for package documentation tools

> <https://github.com/posit-dev/ark/pull/868>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

This change ports some package documentation tools from the `btw` package into ark, exposing them as RPCs that we can call from Positron (where we'll hook them up to language model tool calls).

Specifically, the changes from btw are as follows:

- remove dependency on tidyverse (dplyr, etc.) by rewriting transformations in base R
- return plain-text results rather than throwing errors in most cases
- remove dependency on rmarkdown by bundling the needed pandoc conversion bits 

Source for most RPCs: https://github.com/posit-dev/btw/blob/main/R/tool-docs.R

Part of https://github.com/posit-dev/positron/issues/8016. 


## @jmcphers at 2025-07-08T16:38:42Z

> an error message has an interpolation bug:

Thanks, fixed in https://github.com/posit-dev/ark/pull/868/commits/27b3fbb62210c55409e2b8350764d4e8b73af685!

> Do you think it would be helpful for the rmarkdown and btw package to implement the tools used here in standalone files?

Yes, it would be helpful! I didn't know about this mechanism.

## @lionel- at 2025-07-10T10:58:51Z

@jmcphers I suggested implementing in standalone files in:

https://github.com/posit-dev/btw/issues/78
https://github.com/rstudio/rmarkdown/issues/2596