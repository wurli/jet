# Only emit dynamic plots for console sessions

> <https://github.com/posit-dev/ark/pull/444>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

This change fixes a frequently reported bug in which Jupyter R notebooks in Positron don't show plots in the right location.

The problem is that dynamic plots (which always go to the Plots pane) are always emitted if Positron is connected to Ark. 

The fix is pretty simple; we just _also_ need to be sure we're in console mode before emitting a dynamic plot. 

Addresses https://github.com/posit-dev/positron/issues/3846

## @jmcphers at 2024-07-22T17:18:30Z

It's currently hardcoded to be that size! I feel weird changing it -- surely these values were chosen for a reason -- but you're absolutely right that it looks awkward. 

Updated to use a traditional 4:3 landscape orientation in https://github.com/posit-dev/ark/pull/444/commits/2f63cbcf69d92bdbbb85c3403c2c1a693cd2ba3b.