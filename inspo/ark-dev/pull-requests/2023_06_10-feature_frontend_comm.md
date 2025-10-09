# Send global events over new `positron.frontEnd` comm type

> <https://github.com/posit-dev/ark/pull/27>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

This change sends custom Positron events over `positron.frontEnd` comm (presuming that it exists) instead of over the nonstandard Jupyter `client_event` message. It is a companion to https://github.com/rstudio/positron/pull/724; see that PR for more details.

A helpful side effect is that non-Positron front ends connected to `ark` will not get any Positron events (since they never open a frontend comm). 

## @seeM at 2023-06-12T07:00:38Z

@jmcphers what similar work would we need to do on the Python side?

I'm also curious what features use this comm type. Skimming through amalthea, I see:

- Showing help for an object via the console
- Busy event – I'm not sure what feature this supports
- Show message event – I'm not sure what feature this supports

I don't yet see references to any of these concepts in the Python extension. cc @petetronic 

## @jmcphers at 2023-06-12T15:25:32Z

No work is needed on the Python side for this change -- the stuff getting moved into the comm channel is all NYI in the Python extension (as you noted).