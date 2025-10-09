# Recover from unconforming `format()` methods

> <https://github.com/posit-dev/ark/pull/159>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Branched from #158.
Addresses posit-dev/positron#1862.

`format.Surv()` takes in a matrix and returns a character vector without dimension, which causes us to panic as we were expecting `format()` to preserve dimensions.

To fix this, this PR implements `harp_format()` which checks for assumptions after dispatch. If assumptions are not met, we try hard to recover them: If only the dimensions are missing they are added back. If other assumptions are unmet, we fall back to the default `format()` method.

The PR also adds this supporting infrastructure:

- R modules for Harp on the model of Ark modules. We only have a private namespace that isn't reachable from R at all. This makes it possible to implement `harp_format()` in R. We also use it to initialise state and data for unit tests (the recovery paths are all tested).

- Tweaked registration routines so we can register Harp native functions to call back into Harp from R. This is used to emit Rust log messages from R in case of non-conforming results.


