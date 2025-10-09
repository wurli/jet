# Send pre-renderings of new plots to the frontend

> <https://github.com/posit-dev/ark/pull/775>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Ark side of https://github.com/posit-dev/positron/pull/7247
Progress towards https://github.com/posit-dev/positron/issues/5184
Progress towards https://github.com/posit-dev/positron/issues/6736

We now generate pre-renderings of new plots and send them over to the frontend as part of `comm_open` messages.

The current settings for the pre-renders are stored in a `Cell`. This requires a `Copy` type which is generated on the Positron side (see linked PR).


