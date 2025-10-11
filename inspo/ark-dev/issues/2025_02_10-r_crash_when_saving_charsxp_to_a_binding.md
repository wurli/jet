# R: Crash when saving CHARSXP to a binding

> <https://github.com/posit-dev/ark/issues/692>
>
> * Author: @DavisVaughan
> * State: CLOSED
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABSPDwBw", name = "bug", description = "Something isn't working", color = "E99695")

Likely in the Variables pane

Just run:

```r
x <- rlang:::chr_get("foo", 0L)
```

Without the `x <-` it works fine, but saving to `x` causes a crash

Not likely for many users to do this, definitely expert only to even make one of these, so not super high priority

