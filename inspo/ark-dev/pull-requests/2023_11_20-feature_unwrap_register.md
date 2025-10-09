# Unwrap Rust errors for all registered `.Call` callbacks

> <https://github.com/posit-dev/ark/pull/156>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Implements @jmcphers's idea in https://github.com/posit-dev/amalthea/pull/146#discussion_r1397681610. All functions registered for `.Call()` now must return a `Result`. Rust Errors are automatically converted into R errors at the Rust/R boundary.

In addition, we now sandbox the wrappers to:

- Disable interrupts and condition catching over our Rust stacks.
- Detect unexpected R errors and convert to panics.

I'm a bit concerned that this is disabling interrupts without a timeout mechanism so I left a TODO note about this.

## @lionel- at 2023-11-21T16:32:15Z

@DavisVaughan We now move the return type into the internal lambda passed to `r_unwrap()`, and then replace it by `SEXP` in the function that we are actually registering. Type checking is performed by `r_unwrap()` which takes the function body as input, ensuring that it's a `Result<SEXP, _>` and guaranteeing that the expanded function body does return a `SEXP` type.
