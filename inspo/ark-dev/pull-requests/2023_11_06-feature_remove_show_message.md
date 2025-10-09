# Remove custom `IOPubMessage::Event`

> <https://github.com/posit-dev/ark/pull/138>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

I noticed that our `PositronEvent` handling code was oddly complicated because we had both `send_event()` (which forwards over `ark-comm-frontend`) and `handle_event()` (which sends a custom IOPub message).

We have moved away from the custom IOPub message for everything except `ps_show_message()`, a test hook used to test `R_ShowMessage()` capabilities. This actually isn't even hooked up on the frontend AFAICT (search for `ShowMessageEvent` in Positron itself). I switched this over to use `send_event()`, which is how it should eventually be handled once we support this in the frontend, and it also aligns with how our `r_show_message()` hook in RMain already works. I also removed the usage of globals here in favor of `RMain` (woo!).

This in turn let me remove our custom `IOPubMessage::Event`, so now we are almost 100% in spec for IOPub messages (we have 1 custom `Wait` message that I recently added, but otherwise we are aligned with the spec)

All of this allowed me to remove quite a bit of dead code, which is pretty nice!


---

We have one remaining usage of `R_CALLBACK_GLOBALS` in `ps_editor()` where we need the LSP client object. This isn't part of RMain, so itll be a little more tricky to remove, but it sure would be nice if we could remove that!

