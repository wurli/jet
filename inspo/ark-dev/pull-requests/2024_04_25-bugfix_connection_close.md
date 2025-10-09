# Tell the front-end the connection has been closed on the R side

> <https://github.com/posit-dev/ark/pull/323>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

Adresses https://github.com/posit-dev/positron/issues/2898

If the connection is no longer tracked on the R side, we send a close message to the front-end before interrupting the message loop.

