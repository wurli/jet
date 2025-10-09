# Wrap `R_MAIN` in an `UnsafeCell`

> <https://github.com/posit-dev/ark/pull/664>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Progress towards #661.
Workaround for #663 (see this issue for context).
Supersedes and closes #662.

- Make `R_MAIN` a thread-local variable as this is a nicer way of controlling accesses from other threads.

- Wrap it in an `UnsafeCell` so we're able to create multiple `&mut` to it without compilation warning. This bypasses the borrow checker and causes undefined behaviour. No changes from the status quo in this regard. Alternatively we could pass raw pointers around but that would require lots of `unsafe` markers everywhere to call methods or set state. So accepting the UB is the easiest way to fix the warning until we can do better.

