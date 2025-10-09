# Code Action to "unpipe"

> <https://github.com/posit-dev/ark/issues/823>
> 
> * Author: @DavisVaughan
> * State: OPEN
> * Labels: 

From @EmilHvitfeldt 

> Do we have tools to convert between these two? I do the first one a lot, but i prefer the second in package code

```r
# Before
res <- mtcars |>
  lapply(mean) |>
  unlist() |>
  max()

# After
res <- lapply(mtcars, mean)
res <- unlist(res)
res <- max(res)
```

I'm imagining the user would put their cursor on `res` and get a ✨ Code Action called `Unpipe this chain` or something that would result in the 2nd form.

The nice thing about the code action is that we don't have to show it if we do some analysis and hit a case that feels hard to handle (like, we may or may not decide to support the code action when we see a `_` placeholder, or we may choose not to show it with magrittr pipes, since they aren't pure syntactic changes, or we may choose not to show it if there are comments in the way)

The pipe placeholder is fairly strict in terms of where it can be placed, so that's good

```
> x <- 1
> x |> abs(_ + 1)
Error in abs(x, "_" + 1) : invalid use of pipe placeholder (<input>:1:0)
> x |> abs(_)
Error in abs("_") : 
  pipe placeholder can only be used as a named argument (<input>:1:6)
> x |> abs(x = _)
[1] 1
```

