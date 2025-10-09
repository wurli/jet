# Fix issue causing plot duplication in Positron plot history

> <https://github.com/posit-dev/ark/pull/324>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/2453.

The problem here turned out to be pretty simple: We write snapshots of plots in order to replay them when they are redrawn.  These snapshots were never getting written because our graphics hooks weren't set up correctly -- they used `.ps.call()` which doesn't exist.

The fix is to use the correct casing so that the hooks are invoked. 

## @jmcphers at 2024-04-26T15:14:56Z

> I assume the error was silently swallowed by R?

Yes, it was -- seems to be the standard for hook functions. Maybe a future improvement would be to tryCatch in all our hooks and forward any error conditions to Rust side in a way that it could be logged. 

## @DavisVaughan at 2024-04-26T17:39:17Z

@lionel- had a PR open about this that I guess he forgot about 😛 , ill close it https://github.com/posit-dev/amalthea/pull/302

## @DavisVaughan at 2024-04-26T17:40:30Z

That should mean this is also fixed now https://github.com/posit-dev/positron/issues/2686

## @lionel- at 2024-04-26T18:40:40Z

ugh, completely forgot about it