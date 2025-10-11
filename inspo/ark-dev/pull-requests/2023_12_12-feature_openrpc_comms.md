# Use auto-generated comm contract for Plots and Help

> <https://github.com/posit-dev/ark/pull/181>
>
> * Author: @jmcphers
> * State: MERGED
> * Labels:

This change switches the Plots and Help comms to use the new auto-generated interfaces based on JSON-RPC. There's little behavior change aside from minor changes to message formats; the only really significant change here is that lots of errors that were formerly just logged are now promoted to JSON-RPC errors and sent to the front end.

Full writeup in https://github.com/posit-dev/positron/pull/1942. Requires that PR to be merged.

