# Add `height` arg to viewer, for RStudio compatibility

> <https://github.com/posit-dev/ark/pull/328>
>
> * Author: @juliasilge
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/2900

RStudio's viewer does have a `height` arg:

``` r
getOption("viewer")
#> function (url, height = NULL)
#> {
#>     if (!is.character(url) || (length(url) != 1))
#>         stop("url must be a single element character vector.",
#>             call. = FALSE)
#>     if (identical(height, "maximize"))
#>         height <- -1
#>     if (!is.null(height) && (!is.numeric(height) || (length(height) !=
#>         1)))
#>         stop("height must be a single element numeric vector or 'maximize'.",
#>             call. = FALSE)
#>     invisible(.Call("rs_viewer", url, height, PACKAGE = "(embedding)"))
#> }
#> <environment: 0x1121c74d8>
```

So let's add this now, punting to the future to really handle this in our own Viewer.

With this change, I can now use `testthat::snapshot_review()`:

![snapshot-review](https://github.com/posit-dev/amalthea/assets/12505835/c88aa6a5-7c28-42c1-ae9c-c2907fb0e409)



