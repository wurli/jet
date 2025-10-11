# Disable the LSP on crash

> <https://github.com/posit-dev/ark/pull/679>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

Part of https://github.com/posit-dev/positron/issues/5507

The goal of this PR is to prevent the LSP from ever crashing the R session and lose the user state. I've moved all LSP request handlers behind a [`catch_unwind()`](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html), basically a `try` for panics. When an LSP handler panics, we detect it, report it, and flip our state to crashed. Once crashed, all LSP handlers respond with an error. This causes some chatter in the LSP logs but I made sure these errors do not carry a backtrace to avoid flooding the logs.

Ideally we'd shut down the LSP entirely and forcefully disconnect from the client. Unfortunately that scenario of a server-initiated shutdown is not supported by the LSP protocol and tower-lsp does not give us the tools to do this.

Alternatively we could send a notification to the client that the LSP has crashed. The client could then initiate a shutdown. I chose not to go that route because to avoid having to deal with synchronisation issues and having to make changes to both the client and the server.

For context, this is a temporary workaround. Once the LSP lives in Air a crash will never be a big deal for the user. Most of the time they will not be aware of it since VS Code / Positron silently restarts crashed servers (unless they crash too many times in a short period, in which case the user is notified and the server is no longer restarted).

Here is a screencast of what happens when the LSP crashes:


https://github.com/user-attachments/assets/61737d86-4018-47f9-8a1d-5ed4ad566291


The user is notified of the crash and requested to send a report with the logs.

Note that the relevant backtrace is sent by our panic hook to the _kernel_ logs rather than the LSP logs. The backtrace in the LSP logs is unlikely to be helpful.


## @lionel- at 2025-01-29T08:47:28Z

Davis pointed out that the `serve().await` is not really blocking (not sure how I missed that 😬) so we are able to fully shut down the LSP by waking up a `select`. That's nice because that removes a source of potential problems and that prevents any further log messages in the LSP output channel.

To be on the safe side I decided to keep the crash flag that disables request handlers because our internal notification races with incoming messages from the client so it's possible the main loop will tick again after we detect a crash.

Also I realised we already have the infrastructure to show notifications via Jupyter so I now do that. The downside is that this requires us to go through an `r_task()` to send the notification (it would be possible to avoid that but would require a non trivial amount of plumbing). The upside is that we leave the LSP out of this which seems safer since we are shutting down.
