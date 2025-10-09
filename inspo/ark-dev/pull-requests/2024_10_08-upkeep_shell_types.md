# Streamline handling of Shell replies

> <https://github.com/posit-dev/ark/pull/575>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Some clean ups following discussion in #547.

- Shell handlers now return an `amalthea::Result`.

- New `amalthea::Error::ShellErrorReply` variant for error replies. When a handler returns one, it's forwarded to the client as a Shell reply.

- The `amalthea::Error::ShellErrorExecuteReply` variant is specifically for execution replies as they are the only ones that need an execution count.
 
  (This count is currently created by the R thread. We might want to move some of this handling to the Shell socket: it could create it, bump it when appropriate, etc. The Shell socket might also become in charge of sending Inputs and Results to IOPub to remove as many protocol details from Ark as possible).

- Removed the `ExecuteResponse` enum as we now pass that information through an `amalthea::Result`.

- New `Shell::send_execute_error()` method. Sends errors for execute requests with the execution count.

- `Shell::handle_request()` is now in charge of sending replies from handlers. All errors, including internal (i.e. not the two variants discussed above) are forwarded to the frontend as replies.

