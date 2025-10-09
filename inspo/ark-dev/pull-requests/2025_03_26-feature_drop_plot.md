# Clean up state when plot is closed on the frontend

> <https://github.com/posit-dev/ark/pull/758>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Closes https://github.com/posit-dev/positron/issues/6738
Addresses https://github.com/posit-dev/positron/issues/6702

This was easier than I thought. Since all plots are backed by a Jupyter comm, and since the frontend duly closes clients (the equivalent Positron abstraction corresponding to comms in Jupyter-backed languages), we just needed to handle the close messages that we currently ignore.

We now clean up:

- The list of sockets
- The plot state on the R side
- The currently active device context (for https://github.com/posit-dev/positron/issues/6702)

