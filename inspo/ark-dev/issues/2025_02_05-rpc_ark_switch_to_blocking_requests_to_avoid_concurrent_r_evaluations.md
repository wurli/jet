# RPC: Ark: Switch to blocking requests to avoid concurrent R evaluations

> <https://github.com/posit-dev/ark/issues/689>
>
> * Author: @lionel-
> * State: OPEN
> * Labels:

I think we need to be a bit more careful about the timing of evaluation of backend RPCs on the R side. This concerns the `callMethod()` mechanism (which makes a request from typescript via the UI comm) and more generally any requests performed by a comm. These shouldn't run concurrently with the R interpreter because we are running complex functions that have not been designed for reentrancy or preemption (more precisely interrupt-time, which happens at somewhat controlled points, but in practice we should consider it happens at any time).

RPCs are becoming our main way of calling R code from positron-r because they support serialisation of arguments whereas `execute()` only support textual arguments. Because of this we are bound to be calling complex code, for example code that loads an R package. This is potentially bad for many reasons, for instance if `loadNamespace()` is in the middle of loading another package at that moment.

Same concern for comm RPCs, for instance for the connection comm. The `.ps.connection_` API is accessible from R and might be called by a package or a user. Then a message comes in and runs connection code preemptively (at interrupt time). This means the connection R code should be reentrant but we can't reasonably expect this. I think we'll eventually end up with weird state corruption. It doesn't even require mutable state to get corruption, it just suffices for state to be split in multiple places and change preemptively between two lookups.

Thankfully I don't think we'll lose much by making these requests synchronous. They are currently made via `comm_msg` which is queued on the Shell socket. The problem comes from closing the `comm_msg` request too fast, which allows Shell to invoke the next request, possibly an `execute_request` or another comm RPC. To avoid that we just need to block during the processing of the comm message to prevent `Shell` from executing the next request while the comm is calling R code.


## @lionel- at 2024-04-02T07:40:01Z

Also the propagation and handling of `prompt_signal` events emitted from read-console should be synchronous. These events are used for instance to update the data explorer between top-level commands from the user.

## @lionel- at 2024-05-28T07:08:46Z

Currently ReadConsole pushes prompt events to signal the end of a top-level command. The variable comm listens to these events to refresh the bindings in an `r_task()`, at a time where R might already be running the next command. Instead of pulling the bindings concurrently, we should push the bindings to the comm after each top-level command, the same way we are about to push global scopes to the LSP from ReadConsole.

Or alternatively run a handler on the R tread after each ReadConsole tick to grab the state we're interested in. Could be set up as a new kind of idle task that is guaranteed to run after an iteration.
