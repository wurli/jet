# Skip diagnostics for `library()` calls

> <https://github.com/posit-dev/ark/pull/18>
> 
> * Author: @romainfrancois
> * State: MERGED
> * Labels: 

addresses https://github.com/rstudio/positron/issues/433


## @romainfrancois at 2023-05-31T15:12:57Z

This is very simplistic: it skips diagnostics for calls to `library(<identifier>)` or `require(<identifier>)`, i.e. when there is only one argument used. 

This probably should be more sophisticated and handle cases like: 

```r
library(foo, character.only = TRUE) # in this case, we actually want to check if `foo` is in scope
library(pos = 1, ggplot2)
library(package = ggplot2)
```

Do we have some sort of argument matching logic ? 

## @romainfrancois at 2023-05-31T15:22:49Z

Should this rather be delegated to some R function, similar to what `.ps.completions.getCustomCallCompletions()` for completions ? 



## @romainfrancois at 2023-05-31T15:41:04Z

Another way could be to sit in in `check_symbol_in_scope()` somewhere near: 

https://github.com/posit-dev/amalthea/blob/9727187fe6f2553caa6e59a52284da940b0780d2/crates/ark/src/lsp/diagnostics.rs#L716-L723

and skip if the parent is a "library" call node ?

## @kevinushey at 2023-05-31T19:21:29Z

> Do we have some sort of argument matching logic?

We don't, but this would be very useful. It's probably worth taking some time to discuss the implementation. Some options:

- Translate the tree-sitter "call" node directly into an R function call object, and then use existing tools like `match.call()` to match it.
- Transform the tree-sitter "call" node into our own separate "call" data structure, which gives us a HashMap or similar mapping function names to arguments. We may need to implement our own version of `match.call()` to accomplish this, but this is pretty tricky to get right.

## @romainfrancois at 2023-06-01T12:21:55Z

Thanks @kevinushey. I'll follow up as I build up more understanding of tree-sitter and how to use it here. I refined the code a bit so that it's (IMO) good enough for https://github.com/rstudio/positron/issues/433 for now: 

To go further, we would indeed need some sort of `match.call()` wrapper, I'll work on it as part of https://github.com/rstudio/positron/issues/528