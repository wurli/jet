# Expose environment variables from R process

> <https://github.com/posit-dev/ark/pull/507>
>
> * Author: @juliasilge
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/2723

With this new function (callable via `call_method()` in Positron) we can get the values for a set of environment variables starting with a given regex from the R process:

``` r
.ps.rpc.get_env_vars("POSITRON")
#> $POSITRON
#> [1] "1"
#>
#> $POSITRON_VERSION
#> [1] "2024.09.0"
```

If you need more than one set, you can do something like `.ps.rpc.get_env_vars("POSITRON|VSCODE")`.


## @juliasilge at 2024-09-09T22:25:49Z

@jmcphers also thought that a regex was a bad idea and we should do the work of passing specific env vars by name. This tiny new helper now works like this:

``` r
.ps.rpc.get_env_vars(c("USER", "POSITRON", "POTATO"))
$USER
[1] "juliasilge"

$POSITRON
[1] "1"

$POTATO
[1] ""
```

So basically an unfancy and tiny wrapper around `Sys.getenv()`.


## @juliasilge at 2024-09-11T15:57:42Z

I opened this issue to track a push-based model when the need becomes more urgent: https://github.com/posit-dev/positron/issues/4641
