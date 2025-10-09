# Move low level handling of RPC messages to `CommSocket`

> <https://github.com/posit-dev/ark/pull/188>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

So that comms only need to worry about transforming a request to a reply in terms of their vernacular types without getting into the details of serialisation and response creation.

