# Shut down gracefully if parent exits

> <https://github.com/posit-dev/ark/pull/830>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

This change causes `ark` to gracefully exit if its parent process does, with the goal of preventing ark from becoming a zombie in cases where Kallichore exits abruptly (without shutting down kernels). 

It works by asking the OS to send it `SIGUSR1` (via `PR_SET_PDEATHSIG`) if the parent exits. When we get `SIGUSR1`, we ask the kernel to shut down. 

I thought about making this optional because maybe there are some cases where you might want to leave the kernel running and reconnect to it later? But I can't think of any.

This change is related to https://github.com/posit-dev/positron/issues/7991 and https://github.com/posit-dev/positron/issues/6692, but is not a complete fix for either one. 



## @jmcphers at 2025-06-06T21:28:35Z

@DavisVaughan thanks, that's much tidier!