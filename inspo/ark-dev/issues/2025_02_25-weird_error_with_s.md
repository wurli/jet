# Weird error with `"\s"`

> <https://github.com/posit-dev/ark/issues/722>
> 
> * Author: @lionel-
> * State: OPEN
> * Labels: 

Reported by @hadley 

Evaluating `"\s"` in the console gives a weird error:

![Image](https://github.com/user-attachments/assets/85b467e6-d195-43c7-9e75-f8149ca928b3)

I traced it down to `R_ParseVector()` throwing an error instead of returning a syntax error code. This causes the control flow to end up in this safeguard path: https://github.com/posit-dev/ark/blob/452d00d689c381e51e66edf92ec62b2e13397c66/crates/ark/src/interface.rs#L1335.

## @lionel- at 2025-02-25T12:20:14Z

You'll see the same thing with `1 |> {}`.

In general see https://github.com/r-devel/r-svn/blob/7da17dcbb20d1dfc901efd7dfee1623c21c925cf/src/main/gram.y#L4439 and https://github.com/r-devel/r-svn/blob/7da17dcbb20d1dfc901efd7dfee1623c21c925cf/src/main/gram.y#L4532 for the class of errors that can be thrown by the parser.

We should catch these classed R errors and convert them to proper syntax errors on the Rust side.

## @DavisVaughan at 2025-02-25T12:52:57Z

Why doesn't `R_ParseVector()` return an error code? Should we report that?

## @lionel- at 2025-02-25T16:57:01Z

I don't know but this seems intentional.