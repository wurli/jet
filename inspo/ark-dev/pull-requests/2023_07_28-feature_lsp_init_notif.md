# Send LSP server init notification to frontend

> <https://github.com/posit-dev/ark/pull/71>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

- Move responsibility of sending LSP start notification from `LspComm` to `Shell::open_comm()`.

- The start notification message is now of type `server_started` instead of `lsp_started`. This more general message type will be shared for all comms wrapping a server (e.g. DAP).

- The server notifies `open_comm()` that it is ready to accept connections via a channel. The notification is then forwarded to the frontend through the comm. Before this change we were potentially sending the notification message too soon.

