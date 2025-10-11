# Add integration test for R errors

> <https://github.com/posit-dev/ark/pull/547>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

Branched from #542

I was confused for a while trying to make tests for execution errors because of some puzzling ambiguities:

- In the Jupyter protocol all replies have nominally one type (`foo_reply` with `foo` corresponding to the request name, e.g. `foo_request`), but they each have two different structural types. When the `status` field is "error", all fields are omitted and instead the exception fields (`ename`, `evalue`, `stacktrace`, represented by the Rust type `Exception`) are included.  The contents of the error variants of these messages are represented by the Rust type `ErrorReply`, which currently has `"error"` as `message_type()`.

- As special case, the error variant of `"execute_reply"` messages must also preserve the `execution_count` field.  This is represented by the Rust type `ExecuteReplyException`.

  Ark handles this downstream and for this reason (I think) we don't use `send_error()` in Amalthea's Shell socket, we use `send_reply()` in both cases, which is also confusing: https://github.com/posit-dev/ark/blob/1c7a78f95c187294e17511f76fe10732a993261a/crates/amalthea/src/socket/shell.rs#L235

- Execution errors are also signaled on IOPub with messages of type `"error"`. These are represented by the Rust type `ExecuteError`.

Things changed in this PR:

- I was confused by `ErrorReply` having the same `message_type()` (https://github.com/posit-dev/ark/blob/eef44b6d5fe0dee9f297b530c04a890864ba3fbb/crates/amalthea/src/wire/error_reply.rs#L31) as IOPub's `ExecuteError` messages (https://github.com/posit-dev/ark/blob/eef44b6d5fe0dee9f297b530c04a890864ba3fbb/crates/amalthea/src/wire/execute_error.rs#L24). AFAICT this message type is never used. The payload is included as is by `error_reply()`, see https://github.com/posit-dev/ark/blob/1c7a78f95c187294e17511f76fe10732a993261a/crates/amalthea/src/wire/jupyter_message.rs#L347. It only needs a `message_type()` method for type reasons involving expected traits. So I set the message type to `"*error payload*"` to better reflect it's only a placeholder.

- We now take precautions in the `try_from()` method that deserialises Jupyter messages to disambiguate between the regular and error variants of `"execute_reply"`.

- The error situation is now better documented so it's not as confusing the next time we have to deal with Jupyter errors.

