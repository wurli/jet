# Fixes data explorer tests instability

> <https://github.com/posit-dev/ark/pull/465>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/4222

The problem is definitely related to the Data Explorer calling R from a different thread, but it's made worst because multiple `RDataExplorer::execution_thread()`s were being created but never disposed.

Whenever we start a RDataExplorer, with `RDataExplorer::start()`, we create an execution thread that is only disposed if:

1. receives a CommMsg::Close() (front-ent initiate close)
2. the variable binding is removed from the global environment (this sends a comm close automatically)

But it's possible to create a DataExplorer instance that is not bound to any variable binding, and those will stay alive until they receive comm close. Before this PR we wouldn't send any CommClose msg, resulting in many data explorer threads alive. When sending a `EVENTS.console_prompt.emit(())` all of those threads would execute code that hits the R API at the same time, causing the problems.

We address this by making sure we send CommClose to that thread whenever the socket is disposed.


## @dfalbel at 2024-08-07T16:06:34Z

Yep! FWIW the `r_test` mutex should be enough to protect multiple tests to execute at the same time, so they are essentially already executing sequentially - although maybe in a different order.

https://github.com/posit-dev/ark/blob/224dfe212a58f79cd7612d57af97757c627248f2/crates/harp/src/test.rs#L89-L96


