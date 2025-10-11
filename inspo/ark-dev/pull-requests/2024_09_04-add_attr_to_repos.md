# Add attribute to `repos`, for renv users

> <https://github.com/posit-dev/ark/pull/502>
>
> * Author: @juliasilge
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/4255

You can check out the approach RStudio takes here:
https://github.com/rstudio/rstudio/blob/078e21116b0e34aff92addf961699017adb62fc5/src/cpp/r/R/Tools.R#L698

I saw [your note here](https://github.com/rstudio/renv/issues/1963#issuecomment-2272152739) @DavisVaughan. Given that we are unlikely to be able to change this attribute in RStudio, I don't tend to think we will gain much by using `IDE` or `default`. If you have a strong opinion otherwise, happy to change this, though!

## QA Notes

After this PR is merged and ark is bumped, you will see (edited after code review to change name of attribute):

``` r
getOption("repos")
#>                        CRAN
#> "https://cran.rstudio.com/"
#> attr(,"IDE")
#> [1] TRUE
```

<sup>Created on 2024-09-04 with [reprex v2.1.1](https://reprex.tidyverse.org)</sup>


Notice the new attribute.

Once we get this into a release build, we should update at https://github.com/rstudio/renv/issues/1963.

