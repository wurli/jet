# Add `r_try_catch_any()` to return generic values

> <https://github.com/posit-dev/ark/pull/74>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

New `r_try_catch_any()` variant that returns a generic `Result<R>` value instead of `Result<Robject>`.

Other API changes::

- Rename `r_try_catch()` to `r_try_catch_classes()`.
- Rename `r_try_catch_error()` to `r_try_catch()`

This way the less used class variants gets a longer name, and the more commonly used `RObject` variant gets a short name.

## @lionel- at 2023-08-09T07:15:38Z

I'm just wondering if we should incorporate the `_any` particle in `r_try_catch_classes()` but the name is getting long. Merging now but we can revisit in the future.
