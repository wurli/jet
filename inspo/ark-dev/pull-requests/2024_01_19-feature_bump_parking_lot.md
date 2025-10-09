# New `lock_api` for `parking_lot` has released

> <https://github.com/posit-dev/ark/pull/207>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

We don't actually use the `ReentrantMutex` that this fix was intended for anymore:
https://github.com/Amanieu/parking_lot/pull/390

But I figure it is better to be safe and just bump it anyways.

At the very least it means we don't need the github dep anymore

(the fix happened in the sort-of-internal `lock_api` that `parking_lot` uses, so that's what we have to bump)

