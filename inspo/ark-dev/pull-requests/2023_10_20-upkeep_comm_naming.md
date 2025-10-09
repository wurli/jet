# Renaming proposal for comm API

> <https://github.com/posit-dev/ark/pull/125>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Currently `CommMsg` is used for serialisation and is not user-facing and instead the comms are creating `CommChannelMsg`. I propose to reverse this:

- CommMsg -> CommWireMsg { id, data }
- CommChannelMsg -> CommMsg (Data, Rpc, Close)

Secondly, `CommEvent` has a short name that makes it seem a bit more general than it is. I propose to make it a bit more specific so it's clear it's used for communicating with the comm manager. And similarly rename `CommChanged` to make its purpose clear (update the shell socket state):

- CommEvent -> CommManagerEvent (Opened, Message, PendingRpc, Closed)
- CommChanged -> CommShellEvent (Opened, Closed)

What do you think @jmcphers?

