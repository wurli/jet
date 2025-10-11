# + `match_node_call()` and refine `library()` diagnostics

> <https://github.com/posit-dev/ark/pull/19>
>
> * Author: @romainfrancois
> * State: MERGED
> * Labels:

Related to https://github.com/rstudio/positron/issues/528 https://github.com/rstudio/positron/issues/641

`match_node_call()` converts the ast into a synthetic R call, so that we can match child nodes to the argument names.

In the use case here, this means we can differentiate between:

 - skip the diagnostic for the `package=` and `help=` arguments

<img width="180" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/89d17e9d-dc08-4cf0-868e-fc5294944691">

<img width="306" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/1929646f-274a-4422-8054-918dae8b78c4">

 - unless `character.only = ` is set to `FALSE` in which case we do need to check if the symbol is in scope:

<img width="434" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/261bff41-398b-4acf-9a75-90bc9e4ef031">

i.e. no diagnostic here:

<img width="349" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/46900508-6253-4e2a-9e2a-7b6375fc22d6">



## @romainfrancois at 2023-06-02T12:23:56Z

I'm not sure it's worth avoiding using R's `match.call()` and implement our own argument matching here inspired from R's implementation: https://github.com/r-devel/r-svn/blob/2ea16fcea48b81e432dde557978fa9e0d12ded91/src/main/unique.c#L1832

## @romainfrancois at 2023-06-02T12:29:03Z

Another thing that might be interesting is to check for invalid arguments:

```r
> match.call(library, quote(library(foo, bar = "baz")))
Error in match.call(definition, call, expand.dots, envir) :
  unused argument (bar = "baz")
```

And offer diagnostics for partial names:

```r
> match.call(library, quote(library(pack = dplyr)))
library(package = dplyr)
```

## @romainfrancois at 2023-06-06T13:29:10Z

This might be too complicated and not flex enough, but the idea is to delegate diagnostics to the R side for some known calls, e.g. `library()`:

<img width="375" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/cdbfdd98-cdf2-4fb3-a1a8-11ed3f610211">

<img width="390" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/85820208-674c-44ca-acb3-57a49b208bc1">


## @romainfrancois at 2023-06-16T09:16:39Z

I think this can be merged, so that at least it deals with `library()` and `require()`. I can follow up with something that would deal with some functions in e.g. `ggplot2`, `dplyr`, ... (https://github.com/rstudio/positron/issues/528) but at the end of the day, this probably needs some way for these packages to drive the diagnostics, perhaps via annotations
