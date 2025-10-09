# Simplify environment iterator

> <https://github.com/posit-dev/ark/pull/365>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Remove dependencies on R internals.

## @lionel- at 2024-05-22T16:46:34Z

And now returns a `harp::Result` with a new `MissingBindingError` when the binding no longer exists.