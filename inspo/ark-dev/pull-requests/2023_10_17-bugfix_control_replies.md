# Reply to Shutdown and Interrupt requests

> <https://github.com/posit-dev/ark/pull/115>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/1576.

We don't currently reply to interrupt and shutdown requests even though this is mandated by the protocol. These replies are unused in Positron but might be important for compatibility with other frontends.

For shutdowns, the protocol states that the reply indicates a complete shutdown of the kernel. This is not currently the case in ipykernel AFAICS though, as they send the reply immediately after setting a global flag to be picked up by an event loop. Similarly, this is implemented here by sending the reply as soon as the control handler has processed the request. I should also note that the R and Control threads are racing so there's a good chance that `exit()` is called too soon and no reply is sent. In the future we should be able to do better by implementing the `R_CleanUp()` frontend method and replying from there before exiting.

For interrupts, the protocol does not say much but ipykernel sends the reply right after sending signals, which is what we do too. In any case the interrupt should only be deemed complete when the Shell goes back to Idle, not when the reply is sent. The reply is only an acknowledgement that the kernel did try to interrupt.

## @DavisVaughan at 2023-10-17T19:39:09Z

Relevant spec pages
https://jupyter-client.readthedocs.io/en/stable/messaging.html#kernel-shutdown
https://jupyter-client.readthedocs.io/en/stable/messaging.html#kernel-interrupt