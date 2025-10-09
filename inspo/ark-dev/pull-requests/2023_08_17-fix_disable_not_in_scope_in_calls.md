# Disable "symbol in scope" within call arguments

> <https://github.com/posit-dev/ark/pull/77>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

See also https://github.com/rstudio/positron/issues/528

This is intended to be a temporary stopgap that makes working with NSE based functions like `dplyr::mutate()` or `ggplot2::aes()` or `quote()` less annoying

The idea is to give up on doing the "symbol in scope" diagnostic when recursing into a call's arguments.

The image below demonstrates what we get / give up from this tradeoff. Mainly we lose the ability to flag _real_ symbol in scope issues, like with the `match()` case.

<img width="475" alt="Screenshot 2023-08-17 at 12 03 52 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/0890d1c9-ae6d-47b5-b4dc-10d9d2dd3446">


## @kevinushey at 2023-08-17T16:25:59Z

Would it be worthwhile to have something less aggressive here? Since almost all variable usages will be within function calls.

We could set a flag whenever we encounter a function call that we "know" performs NSE? RStudio does two things:

1. A hard-coded list of NSE primitives -- https://github.com/rstudio/rstudio/blob/9ab1a4202392e3481446d13f84d455559ab47e10/src/cpp/session/modules/SessionCodeTools.R#L1491-L1495,

2. Some introspection on function bodies, e.g. https://github.com/rstudio/rstudio/blob/9ab1a4202392e3481446d13f84d455559ab47e10/src/cpp/session/modules/SessionCodeTools.R#L1497-L1513

## @DavisVaughan at 2023-08-17T16:51:15Z

Just a note that this exact code doesn't work right for `dplyr::mutate()` because `dplyr:::mutate.data.frame` actually uses `dplyr_quosures(...)` rather than something like `enquos(...)` to capture the vars.

<img width="242" alt="Screenshot 2023-08-17 at 12 35 07 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/de69abe9-bb04-440f-9071-0582c3b3655c">

Also, this "symbol in scope" diagnostic is off in rstudio by default

---

I imagine there are also other cases where the `...` are passed untouched through various layers of function calls before finally being captured with `enquos(...)`. It works a little better with named variables where you have to do `enquo(x)` or `{{ x }}` to pass them through - even at the top level

Although even with named variables there are weird cases, like ggplot2's capturing of `x` and `y` here

```r
> ggplot2:::aes
function (x, y, ...) 
{
    xs <- arg_enquos("x")
    ys <- arg_enquos("y")
    dots <- enquos(...)
```

## @kevinushey at 2023-08-17T17:26:50Z

I think the "right" solution is to have packages declare how their arguments are evaluated in some way that could be consumed by front-ends, but that's a larger project...

Either way, this PR LGTM, just wanted to raise some of the simpler steps we could take to roll this back in the future.