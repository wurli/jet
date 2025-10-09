# Refactor `finish_request()` into result and error paths

> <https://github.com/posit-dev/ark/pull/53>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

This is mostly a pure refactor of `finish_request()` into the result vs error paths.

The only behavior change is that now the error path no longer tries to emit an `IOPubMessage::ExecuteResult`. This seems to mostly be a placeholder right now, but looks to only be applicable for the result path.

In a follow up PR I'll add an `IOPubMessage::ExecuteError` to the error path, since Jupyter front ends seem to expect those

