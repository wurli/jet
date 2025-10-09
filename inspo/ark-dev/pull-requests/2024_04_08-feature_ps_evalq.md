# Add support for symbols in `ps.internal()` to retrieve private objects from namespace

> <https://github.com/posit-dev/ark/pull/301>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

This PR adds a small nicety for debugging ark. Simple symbols are now evaluated in the private namespace by `.ps.internal()`. Previously passing a symbol would fail because we'd try to subset it as if it were a call. With the new behaviour we can easily pluck objects from the namespace. For instance `.ps.internal(env_unlock)` retrieves the private function `env_unlock`.

