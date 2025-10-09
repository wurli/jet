# Add showPrompt API handlers

> <https://github.com/posit-dev/ark/pull/920>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

Adds API handlers for `rstudioapi::showPrompt` and creates a UI event of the name name.

The only interesting thing here is that the RStudio implementation of this method implements the timeout by (theoretically) limiting the amount of time the backend waits for the frontend. In Positron, we implement the timeout entirely on the frontend. 

In practice, it doesn't look like the `timeout` parameter works at all on RStudio, so the difference is probably academic. 

Part of the fix for https://github.com/posit-dev/positron/issues/9099.

