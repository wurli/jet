# Ark: Coercion methods into `RObject` vectors should catch OOM errors

> <https://github.com/posit-dev/ark/issues/693>
> 
> * Author: @lionel-
> * State: OPEN
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABSPDwBw", name = "bug", description = "Something isn't working", color = "E99695")

The `allocVector()` calls in vector coercion methods (e.g. https://github.com/posit-dev/amalthea/blob/f16a0137ed416330658ffd242c2b8c7b5792f045/crates/harp/src/object.rs#L759) might throw an R error over our Rust stack. We should run these in a `r_try_catch()`, probably via a wrapper that returns a `Result`.

