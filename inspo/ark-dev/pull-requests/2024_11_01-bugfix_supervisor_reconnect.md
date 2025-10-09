# Avoid panics due to LSP connectivity issues

> <https://github.com/posit-dev/ark/pull/617>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

This change addresses an issue in ark when reading LSP events and sending notifications. If these operations fail, the kernel crashes and all user data is lost. 

The change minimally avoids these crashes as follows:

- If a notification can't be sent, we log instead of panicking.
- If the event loops don't receive events, we end the loops instead of panicking. 

Addresses https://github.com/posit-dev/positron/issues/4959.



## @jmcphers at 2024-11-04T16:16:16Z

> Unless I'm mistaken the changes in main_loop.rs should not be necessary. In principle I'd prefer to document invariants than creating safety handling that never gets called.

They shouldn't be necessary, but the changes are not speculative -- I observed a couple of runtime crashes on these lines (in the `.unwrap()` that I'm adding error handling too) before I made the change. 

## @jmcphers at 2024-11-07T18:58:11Z

OK, thanks so much for the review and discussion!