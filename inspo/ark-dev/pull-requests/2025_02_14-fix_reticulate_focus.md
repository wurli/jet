# Fix reticulate focus

> <https://github.com/posit-dev/ark/pull/713>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

When there's already a comm open for reticulate, we need to send an event message to the front-end:

https://github.com/posit-dev/ark/blob/1eb4a8d9e9eab8211bc87f0356e611155662d435/crates/ark/src/reticulate.rs#L105-L118

I don't know of a direct way of doing this. But here, we send a message to the comm, and then forward it to the front-end
through the outgoing_tx.

This will help addressing https://github.com/posit-dev/positron/issues/3865, as in that case, the quarto extension will call `real_python(input=<chunk_code>)` and we'll forward that code to the front end so that we can execute within the reticulate Python session.


## @dfalbel at 2025-02-19T13:55:29Z

The message is now sent directly to the from-end with https://github.com/posit-dev/ark/pull/713/commits/4695ccf93c3fef633ca11fcdce966faf2f3e40a5 and https://github.com/posit-dev/ark/pull/713/commits/52895fc3eca8d73593d89e16c5efd671907cda81

It might still not be ideal, because we have to clone the service in order to send it to the execution thread. We could avoid cloning by introducing a different crossbean channel like we do in the UI channel:

https://github.com/posit-dev/ark/blob/52895fc3eca8d73593d89e16c5efd671907cda81/crates/ark/src/ui/ui.rs#L48

But this seemed an overkill, instead, we cleanup the global when the thread is closed:

https://github.com/posit-dev/ark/blob/52895fc3eca8d73593d89e16c5efd671907cda81/crates/ark/src/reticulate.rs#L83-L87



## @dfalbel at 2025-02-25T11:17:08Z

I had an intermediate approach where I stored to the `comm` socket (and thus the necessary channels) in the global: https://github.com/posit-dev/ark/pull/713/commits/4695ccf93c3fef633ca11fcdce966faf2f3e40a5

Does this sound right to you? Are there problems in storing a clone of the comm socket and calling it from the R thread (potentially at the same time as the service thread)? It seemed fine to me, but I don't know about the socket internals. 

## @dfalbel at 2025-02-25T16:24:07Z

I have updated the PR to keep a gloabl reference to the socket. I'm not sure it's only accessed only by the R thread, because we need to clean it up when the current reticulate execution thread isclosing. So I think, we'll still need a mutex.



## @dfalbel at 2025-02-25T17:10:35Z

@lionel- I have updated to keep just the outgoing_tx channel in global: https://github.com/posit-dev/ark/pull/713/commits/3e31307acc8478dd747984ee7cd07989d01286c2