# Use a `ReentrantMutex` for the R lock

> <https://github.com/posit-dev/ark/pull/48>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Closes https://github.com/posit-dev/amalthea/pull/32 (alternative approach).

This switches to a recursive mutex in `r_lock!` known as a `ReentrantMutex` in parking_lot:
https://docs.rs/lock_api/latest/lock_api/struct.ReentrantMutex.html

The main two changes in a reentrant mutex are:
- Locking multiple times from the same thread will work correctly instead of deadlocking.
- `ReentrantMutexGuard` does not give mutable references to the locked data.

The second bullet is currently not applicable to us, because we don't actually carry any data in the mutex. We eventually will if we implement my `RApi` struct idea, but that won't need to be mutable.

The first bullet means that we can now call `r_lock!` within an `r_lock!` without creating a deadlock. This is already pretty useful to be able to do, but it also will help with implementing the `RApi` struct idea, because it should mean we won't have to pass the `RApi` struct through function signatures quite as much. We should just be able to start an `r_lock!` block to get access to the `RApi`, even if we are already inside another `r_lock!`.

This change does mean that we can get somewhat confusing logs if we nest `r_lock!`s:

<img width="735" alt="Screen Shot 2023-06-20 at 3 05 17 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/21de71b2-0ef5-48a2-a426-0259f6ce62dd">

To account for this, I've added an atomic "nest level" integer so we can at least get this instead:

<img width="855" alt="Screen Shot 2023-06-20 at 4 26 07 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/bfd5521b-5f9d-49f8-8135-5752edbc6958">


The `ReentrantMutex` API does have `is_owned_by_current_thread()` (https://docs.rs/lock_api/latest/lock_api/struct.ReentrantMutex.html#method.is_owned_by_current_thread) which would be helpful for one layer of nesting here, but unfortunately it wouldn't be useful for multiple layers of nesting. 

The internal `ReentrantMutex` API holds onto a `lock_count` which is basically the same as my nest level, but there is no API to access it sadly. https://docs.rs/lock_api/latest/src/lock_api/remutex.rs.html#164

