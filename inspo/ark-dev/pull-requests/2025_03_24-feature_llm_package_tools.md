# Add rpc to get loaded packages

> <https://github.com/posit-dev/ark/pull/752>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

Ark side of https://github.com/posit-dev/positron/pull/6954; just adds an RPC to get the loaded packages, and calls `unlist` on the set of packages to install since those arrive in list form from the front end. 

