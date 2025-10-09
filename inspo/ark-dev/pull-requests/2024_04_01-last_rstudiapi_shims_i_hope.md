# Add a few more OpenRPC methods for rstudioapi shims

> <https://github.com/posit-dev/ark/pull/286>
> 
> * Author: @juliasilge
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/1312 with what I hope 🤞 are the last Public Beta rstudioapi shims

Goes along with https://github.com/posit-dev/positron/pull/2593

## @juliasilge at 2024-04-01T22:16:47Z

To test this out, together with https://github.com/posit-dev/positron/pull/2593, try:

```r
rstudioapi::sendToConsole("1 + 1")
rstudioapi::sendToConsole("1 + 1", focus = FALSE)
rstudioapi::sendToConsole("1 + ")
rstudioapi::sendToConsole("lm(mpg ~ ., data = mtcars)")

some_code <- c("library(tidyverse)", "arrange(mtcars, mpg)")
rstudioapi::sendToConsole(some_code, focus = FALSE)
rstudioapi::documentNew(some_code)
rstudioapi::documentNew("SELECT * FROM 1", type = "sql")
```

## @juliasilge at 2024-04-01T22:34:22Z

Addresses https://github.com/posit-dev/positron/issues/2441 because I figured out what to do.

Short answer: the rstudioapi shims must have exactly the function signatures as [from this file](https://github.com/rstudio/rstudio/blob/main/src/cpp/r/R/Api.R).