# Add a `--startup-file` ark argument

> <https://github.com/posit-dev/ark/pull/78>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Linked with https://github.com/rstudio/positron/pull/1106

@lionel- and I talked with Gabor and determined that the source of https://github.com/rstudio/positron/issues/1057 and https://github.com/rstudio/positron/issues/1032 is the fact that we set `R_CLI_NUM_COLORS` as an environment variable on startup.

Environment variables are inherited in subprocesses, i.e. like the one created by `devtools::document()`. The `R_CLI_NUM_COLORS` option has pretty high priority in cli (in particular, higher than `crayon.enabled = FALSE`, which is set when evaluating R code while documenting https://github.com/r-lib/roxygen2/blob/a34bdf7c7b952068b1f7742b7b34d1eac78e54da/R/rd-eval.R#L17), so that explains https://github.com/rstudio/positron/issues/1057

I'm not 100% sure why it shows up in Julia's snapshot tests, it looked to me like `testthat::local_reproducible_output()` sets `cli.num_colors` which should have had higher priority than the envvar.

I was able to reproduce the issue with `devtools::build_readme()` though, which I don't think tries to set any color related options (so the env var goes into the subprocess).

---

Our solution is to instead set these as _global options_, which aren't inherited by subprocesses.

We could hardcode these global options in ark itself, but ark could be used outside Positron in a context that doesn't support these cli options, so instead we need to let Positron tell ark about them somehow. To do this, we've added a new ark argument called `--startup-file` which takes an optional R file to call after the R session has started up. `positron-r` owns the R script and just tells ark about it. The R script contains `options()` calls to set up the cli global options.

---

- Reprex works (see video)
- `devtools::document()` works on vctrs now
- `devtools::build_readme()` no longer puts color related ANSI in the output

https://github.com/posit-dev/amalthea/assets/19150088/cb67da39-c47a-4b8a-9bc0-49bd92722815



## @DavisVaughan at 2023-08-18T18:12:12Z

@juliasilge would you mind testing if https://github.com/rstudio/positron/issues/1032 is fixed for you with this? You need:

- This amalthea PR
- This Positron PR https://github.com/rstudio/positron/pull/1106
- The dev version of cli

With all that, hopefully these issues go away

---

Oh you may have a little trouble getting the amalthea PR to work right if you aren't used to this workflow. When you start Positron you need to open the `amalthea/` folder in `extensions/positron-r` in that Positron session. Then `CMD+SHIFT+B` should build the Rust related bits related to these changes. Then if you close Positron and re-open it you should have everything you need to test it.

If you just run `yarn` from vs code then it will build the Rust bits too, but it first switches your amalthea git branch to the commit that Positron is pinned to, which isn't this PR

## @juliasilge at 2023-08-18T21:17:01Z

This works great now! 🎉

- Running snapshot tests works ✅
- `devtools::build_readme()` only generates nice output
- I can use reprex -- this reprex was generated from inside Positron:

``` r
library(parsnip)

## note that `bart()` has a pretty nasty namespace collision:
tidymodels::tidymodels_prefer()

spec <- bart(mode = "regression")
fitted <- fit(spec, mpg ~ ., mtcars)

predict(fitted, new_data = head(mtcars, n = 3))
#> # A tibble: 3 × 1
#>   .pred
#>   <dbl>
#> 1  20.7
#> 2  20.7
#> 3  24.5
predict(fitted, new_data = head(mtcars, n = 3))
#> # A tibble: 3 × 1
#>   .pred
#>   <dbl>
#> 1  20.8
#> 2  20.7
#> 3  24.7
predict(fitted, new_data = head(mtcars, n = 3))
#> # A tibble: 3 × 1
#>   .pred
#>   <dbl>
#> 1  20.9
#> 2  20.7
#> 3  24.7
```

<sup>Created on 2023-08-18 with [reprex v2.0.2](https://reprex.tidyverse.org)</sup>
