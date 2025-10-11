# Use an `RwLock` for `AUXILIARY_EVENT_TX`

> <https://github.com/posit-dev/ark/pull/668>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Part of https://github.com/posit-dev/ark/issues/661

We expect it to be write locked very rarely, so it should still be pretty cheap to get access to this safely

As we saw in tracing-subscriber, this is the same kind of mechanism that the layer "handle" uses internally, so hopefully the overhead is quite low.

