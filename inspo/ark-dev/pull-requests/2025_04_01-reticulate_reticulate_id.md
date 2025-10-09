# Add a method to query a unique session id

> <https://github.com/posit-dev/ark/pull/759>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

This is part of https://github.com/posit-dev/positron/pull/7024
Each R session can only ever host a single reticulate session. With this PR we add a `reticulate_id()` method that allows us to link a Positron `sessionId` to the ark session it's running. This is necessary specially when a session is opened from Positron directly, (not calling `reticulate::repl_python()` ) - which currently doesn't trigger the reticulate service to be started.

