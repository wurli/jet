# Zed can't print the results using REPL cells

> <https://github.com/posit-dev/ark/issues/788>
> 
> * Author: @aymennasri
> * State: OPEN
> * Labels: 

When running code within REPL cells, ark no longer sends the results only displaying a `Connecting...` or `Queued...` text despite the successful code execution demonstrated by using a function that creates/saves files like `ggsave()` in R.


## @i2z1 at 2025-05-18T15:33:44Z

I've got the same behavior. But sometimes it responds with this message:
`Kernel process exited with status: Exit Status(unix_wait_status(134))`
I am using Archlinux with Zed 0.186.9 and Ark 0.1.184 but Ark version 0.1.159 has the same behavior as mentioned.

## @mmyrte at 2025-08-05T16:27:54Z

I got this regression bug when trying to upgrade from ark `0.1.164` to `0.1.201`, if this helps to narrow things down. I'm running macOS.

I do not have the time to exhaustively test every version in between. I tried the `--log` flag to look at logs, also with higher log levels. I could not even get the "debug" release to run. Would be glad to help test, but don't know how to do so efficiently.