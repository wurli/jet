# Use fewer threads to spare memory

> <https://github.com/posit-dev/ark/pull/720>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Branched from #719.
Addresses https://github.com/posit-dev/positron/issues/5050

By default each tokio worker thread gets 2mb of stack space. This PR reduces the number of threads used by tokio:

- Use a single worker thread in the help proxy server
- Use two worker threads in the LSP as well as 2 blocking threads for diagnostics

With these changes I get from 124 mb to 114mb.

![Screenshot 2025-02-24 at 11 35 58](https://github.com/user-attachments/assets/1237dca4-36aa-48fa-8215-93704897d2d1)

In principle it would make sense to have the proxy server share the runtime with the LSP but probably not worth doing this work since we plan for the LSP to move from tower-lsp to lsp-server, at which point it will no longer rely on tokio.

