# Type `NFRAME_CALL` with an `Option<SEXP>`

> <https://github.com/posit-dev/ark/pull/61>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Minor tweak as followup to looking at https://github.com/posit-dev/amalthea/pull/51/

It seemed like we used a `usize` because we can't statically initialize a `SEXP` with

```
static mut NFRAME_CALL: SEXP = R_NilValue;
```

but we can use an `Option<SEXP>`.

This was a little tough for me to understand when I read it the first time, so I think it is worth switching to this just for clarity

## @lionel- at 2023-07-06T17:28:18Z

By the way calling these functions from another thread might require going back to `usize`
