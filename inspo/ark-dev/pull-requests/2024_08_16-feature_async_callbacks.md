# Async callbacks for processing GetColumnProfile RPC requests

> <https://github.com/posit-dev/ark/pull/473>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

This PR pairs with https://github.com/posit-dev/positron/pull/4326
And should be merged after posit-dev/positron#458

This makes `GetColumnProfile` requests asycronous giving the data explorer execution thread oportunities to compute other RPC methods, such as GetDataValues which should make the data explorer UX a little smoother.

## @lionel- at 2024-08-21T12:10:01Z

@dfalbel I changed the base to `feature/histograms-frequency-tables` but I still get unrelated diffs. I'll review nonetheless using this commit as a guideline since it seems that's the only relevant one here: [Use async callbacks allowing other RPC methods to execute between get…](https://github.com/posit-dev/ark/pull/473/commits/2fb74ba277448a98ea347436b7495f0b393c6a5b)

## @dfalbel at 2024-08-21T13:05:01Z

@lionel- I rebased on the histograms branch and It's now correct. 
But yes, https://github.com/posit-dev/ark/pull/473/commits/985a7d98cd5cb3ece103275ea0ef40a5d8d30c4d is the only relevant commit, the other commit is just fixing tests.

## @dfalbel at 2024-08-30T15:34:10Z

Updated to use `spawn_idle`:

The most challending thing was that the task in `spawn_idle` can live longer than the data.frame we are currently visualizing, so the Rust compiler won't allows us to move that object to a different thread. It's also not safe to clone the table, in the rpc handler thread because it requires calling the R API. So similarly to the `VDOCS` table, we now store references to the currently visualized table in a global `DATA_EXPLORER_TABLES` `DashMap` and we created a wrapper `Table` object that abstracts away how the access to the RObject. It keep the requirements from `RThreadSafe<>` and can only be accessed from the main R thread.

Moved all profile handling functions to a separate file as they can no longer depend on `self`, since they are called from a thread that might outlive `self`.

I also had to change tests, because since there's no spawned thread in the tests, the tasks are now running synchronously and thus the messages get out of order.
