# Improve lifecycle management of LSP

> <https://github.com/posit-dev/ark/issues/622>
> 
> * Author: @DavisVaughan
> * State: OPEN
> * Labels: 

@lionel- and I were looking at the LSP lifecycle a little more and realized that when the tower-lsp server exits here:

https://github.com/posit-dev/ark/blob/b8505c504eb10be0e9cb948e1631f151825facdb/crates/ark/src/lsp/backend.rs#L427

we should also _unset_ the events channel in `RMain` that we sent it here:

https://github.com/posit-dev/ark/blob/b8505c504eb10be0e9cb948e1631f151825facdb/crates/ark/src/lsp/backend.rs#L397-L403

because there is no longer anything it can send messages to.

The tower-lsp server will exit like this during a Positron developer refresh, so it is a decently common case. I think it is possible for us to get in a bad place if we don't clear the `RMain` sender.

## @lionel- at 2024-11-08T07:27:58Z

See discussion in https://github.com/posit-dev/ark/pull/617#discussion_r1827556541

## @lionel- at 2024-11-08T07:33:22Z

If we solve the issue of zombie extension hosts, it would also be good to detect whether an LSP is already running on `comm_open`. That would be an error as the client should first shut down the LSP.

Unfortunately `comm_open` is a notification, not a request, so there is no straightforward way of propagating the error. We might want to extend this message and allow it to be a request if a field is set. This would be advertised as a kernel capability and allow the client to wait for a result and know for sure whether an LSP was started.