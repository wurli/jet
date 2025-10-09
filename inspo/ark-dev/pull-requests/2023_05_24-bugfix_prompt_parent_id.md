# Assign parent ID to input requests from kernel

> <https://github.com/posit-dev/ark/pull/8>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses rstudio/positron#534.

Currently the activity prompt items created on input requests from the kernel (e.g. `readline()`) do not have the correct parent. This causes the out-of-order rendering issues described in rstudio/positron#534. This PR fixes this by keeping track of the id of execution requests from inputs (e.g. a `readline()` call supplied by the user) and setting these as parents of the input requests from the kernel.

Since `JupyterKernel::emitMessage()` already sets parent IDs if a `JupyterHeader` is stored in the parent field of a message, we just need to pass the header to `JupyterMessage::create_with_identity()` and store it as parent.

As before we also need to pass along the 0MQ ID for routing. Instead of passing a whole `JupyterMessage`, a new `Originator` struct is constructed that contains only the elements that we need. This avoids a generic parameter for the message content and abstracts away the JupyterMessage details. The struct is passed by value (clone) to avoid having to include reference lifetimes in all the enums, structs, and functions that directly or indirectly reference the struct (e.g. via the `Request::ExecuteCode` enum variant).


