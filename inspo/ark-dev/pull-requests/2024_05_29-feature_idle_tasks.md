# Spawn idle R tasks that don't run at interrupt time

> <https://github.com/posit-dev/ark/pull/371>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Branched from #365.

Closes posit-dev/positron#2284.

Adds `r_task::spawn()` which takes a future (e.g. the result of an async function) and an executor of futures in our ReadConsole loop. This allows running code non-concurrently with R, and makes it possible to break the computation in small increments by yielding regularly to ReadConsole with `await`.

As part of this work I refactored the ReadConsole event loop to extract the handlers out of the `select!` and simplify our task queue which is now a simple channel.

I've renamed `r_task_async()` to `r_task::spawn_interrupt()`. Should we rename `r_task()` to `r_task::block_interrupt()`?

## @lionel- at 2024-06-03T11:06:25Z

@DavisVaughan Tasks are now protected more consistently thanks to your observations.

I've also changed our RAII structs to make it easier to create new ones and so they all use the same implementation. Could you review this as well please, it's a little tricky.

## @lionel- at 2024-06-03T11:35:08Z

I've renamed `spawn()` to `spawn_idle()`. We can rename `r_task()` to `block_interrupt()` in an ulterior PR.