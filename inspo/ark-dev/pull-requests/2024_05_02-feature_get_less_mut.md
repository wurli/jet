# Only use `get_mut()` when we absolutely have to

> <https://github.com/posit-dev/ark/pull/338>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

In theory this frees up the LSP so it can run some of these requests more concurrently (as long as they don't require R, which will also block).

The `DashMap` allows:
- Many `get()` calls, as long as there have not been any `get_mut()` calls
- Exactly 1 `get_mut()` call

i.e. like an RW lock https://doc.rust-lang.org/std/sync/struct.RwLock.html

