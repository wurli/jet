# Respond with `InternalError` when handler fails

> <https://github.com/posit-dev/ark/pull/231>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

I noticed that the plot errors caused by the issue described in https://github.com/posit-dev/positron/pull/2196 were not correctly typed. With this fix:

- We now respond with `InternalError` when a message handler fails instead of `MethodNotFound`.
- `MethodNotFound` is now correctly sent when `serde_json` can't find a corresponding request type.

