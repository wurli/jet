# Reticulate: fix deadlock caused by not disposing of the guard before calling `start`.

> <https://github.com/posit-dev/ark/pull/727>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

Made a bad mistake when updating: https://github.com/posit-dev/ark/pull/713
We need to acquire the lock in `start()` thus, we need to make sure we release it before calling ` start()`.

