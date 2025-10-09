# Bugfix: folding ranges within brackets

> <https://github.com/posit-dev/ark/pull/842>
> 
> * Author: @kv9898
> * State: MERGED
> * Labels: 

This partly addresses https://github.com/posit-dev/positron/issues/8059, where folding ranges within brackets do not work normally.

Before this fix, code such as:
```r
foo <- function(
  x = "}"
) {
  # inside level 1 ####
  b = 2
  b <- b |> # indentations should also be folded
    summary()
  # another level 1 ####
  hi <- 2
}
```
would see only the first comment section folded properly, but not the second one. After this fix, both would be folded.

## @lionel- at 2025-06-23T09:54:49Z

@kv9898 Thanks for the fix! Just needed a unit test