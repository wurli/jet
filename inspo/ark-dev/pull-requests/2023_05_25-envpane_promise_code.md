# fill display_value with  PRCODE for unevaluated promises

> <https://github.com/posit-dev/ark/pull/10>
>
> * Author: @romainfrancois
> * State: MERGED
> * Labels:

With

```r
delayedAssign("x", rnorm(10))
```

This gives:

<img width="279" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/7727425a-1e80-4065-82c0-cb89a8af4a66">

and then when forced:

<img width="554" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/08f17b2a-b2d9-4208-b16a-b627e6c1423c">


## @romainfrancois at 2023-05-25T16:06:54Z

rstudio displays unevaluated promises greyed out:

<img width="464" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/ef8e6548-da0d-4ed1-bffb-bae0c7f784d2">

and offers a way to run them: it calls `force()` when we click on them. I don't know if we want to take inspiration from this.

## @romainfrancois at 2023-05-25T16:13:44Z

I kind of like the greyed out value to express that `x` is not ready yet, but this is the code that would run to get the value.

@petetronic should this be part of the `EnvironmentVariable` message somehow ? Or perhaps `ValueKind` could gain a `Lazy` or `Promise` variant so that the ui can adjust.

## @kevinushey at 2023-05-25T18:44:19Z

FWIW I agree that we should have some alternate display here (greyed out? italics?) just to help separate it from, for example, a character vector with the contents `rnorm(10)` or even a plain call object like `quote(rnorm(10))`.

## @DavisVaughan at 2023-05-25T19:03:44Z

I think this addresses https://github.com/rstudio/positron/issues/627

If you look at the RStudio code linked in that PR, it is pretty complicated to avoid formatting / deparsing something that is "too long"

## @romainfrancois at 2023-05-30T15:32:52Z

Merging this now before #11 but will potentially follow up
