# Data explorer - basic support for list columns

> <https://github.com/posit-dev/ark/pull/356>
>
> * Author: @dfalbel
> * State: MERGED
> * Labels:

This PR adds basic support for list columns in the data explorer.
Addresses https://github.com/posit-dev/positron/issues/1432

The display is inspired by how tibbles are printed to the console. In the future we want to add something similar to RStudio allowing to inspect individual list elements in a new window.

Here's a test case displayed in RStudio and Positron for comparison:

```
list_cols <- tibble::tibble(
  list_col = list(c(1,2,3,4), tibble::tibble(x = 1, b = 2), matrix(1:4, nrow = 2), c(TRUE, FALSE))
)

View(list_cols)
```

![image](https://github.com/posit-dev/amalthea/assets/4706822/1d58a3c5-c4b1-4fba-90e3-fc4be6403dff)

![image](https://github.com/posit-dev/amalthea/assets/4706822/005fddb5-3460-463d-9a4f-000d6032dee2)

We currently intentionally do not support classed list columns (such as data.frame cols) as `length(col)` wouldn't necessarily match the number of rows in the data.frame.


## @dfalbel at 2024-05-28T22:09:16Z

Sure, I'm not sure what's the best way to do it though:

- I could change `r_format()` to return a `CharacterVector` instead of `Vec<String>`, but then we need something like `RObject::from(r_format(x)?.data()).try_into()?` to convert to a `Vec<String>` which is not pretty.

- Another option is to make `r_format()` return an `RObject` or a `SEXP`, but then it will be up to the user of `r_format()` to check it's actually a `Vec<String>` which is what's should be expected - the `FormattedVector` interface will do this as it needs a CharacterVector.

It feels to me that `Vec<String>` is the correct return type, but changing `FormattedVector` to use it requires a non significant refactor. What do you think?

I could also just call `harp_format()` for `r_data_explorer.rs` but felt it's not ideal as I see no calls to harp `HARP_ENV` inside ark.

## @DavisVaughan at 2024-05-30T14:48:24Z

Oh @dfalbel I think it should be `pub fn r_format(x: SEXP) -> RObject {` or even `-> SEXP`, which I don't mind, but the caller just assumes that the function has a guarantee that it returns a character vector (which `harp_format()` on the R side should guarantee, and it does I think). You could add a documentation note above `r_format()`'s implementation about its return value type too.

## @DavisVaughan at 2024-05-30T14:48:43Z

Hit the wrong button oops

## @DavisVaughan at 2024-05-30T14:50:25Z

In general I'm typically a fan of `SEXP` in, `SEXP` out, for functions that live in `utils.rs`, and then callers of those low-level-ish functions build extra support on top of that. That typically gives you the most flexibility because sometimes you need the `SEXP` and sometimes you need to cast it to `.to::<Vec<String>>()`, but it is pretty hard to know when that will be when designing functions like `r_format()`, so more general and flexible tends to be better.

## @dfalbel at 2024-05-31T12:15:05Z

Thanks! It makes sense to keep `utils.rs` as flexible as possible.
I've updated it to return a SEXP and also using it in the FormattedVector context too.
