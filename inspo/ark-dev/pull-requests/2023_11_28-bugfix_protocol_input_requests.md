# Nest input requests inside execute requests

> <https://github.com/posit-dev/ark/pull/165>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

- Input requests in `read_console()` no longer cause the current `execute_request` to be completed with a reply and a transition to Idle state. They are now fully nested in the current execute request. This is consistent with the Jupyter protocol, the ipykernel implementation, and prevents Ark from appearing Idle when it really is in the middle of a computation, and shouldn't receive arbitrary execution requests at that point (e.g. from foreground R tasks).

  Addresses posit-dev/positron#1701, posit-dev/positron#1838, posit-dev/positron#1761

- Amalthea's input-reply shell handler has been replaced with a transmission channel that directly communicates with StdIn. The channel, passed to `Kernel::connect()` as `input_reply_tx` on startup, is directly analogous to `input_request_rx` which is already used to communicate requests to the StdIn thread.

  This removes the Shell thread as an intermediate actor between the R and StdIn threads and simplifies the concurrency structure. This change also was necessary because Shell is blocking while an execute request is active, and so is no longer available to handle input requests since we're no longer closing the active execute request when input is requested.

