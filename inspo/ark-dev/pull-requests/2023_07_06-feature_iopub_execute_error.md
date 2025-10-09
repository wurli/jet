# Emit `IOPubMessage::ExecuteError` in execution error case

> <https://github.com/posit-dev/ark/pull/62>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Part of https://github.com/rstudio/positron/issues/786

First commit reapplies part of https://github.com/posit-dev/amalthea/pull/53, which we accidentally lost in #57 (specifically about only emitting `IOPubMessage::ExecuteResponse` in the success case)

Second commit is specifically about:

> In a follow up PR I'll add an IOPubMessage::ExecuteError to the error path, since Jupyter front ends seem to expect those

mentioned at https://github.com/posit-dev/amalthea/pull/53#issue-1771853594

---

Ignore the logging, but now we at least see the red error blocks in jupyter-lab. There is extensive discussion in https://github.com/rstudio/positron/issues/786 about more improvements that are required here, but having them show up at all is the first step.

<img width="1337" alt="Screenshot 2023-07-06 at 1 53 57 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/3ca2c9f5-e584-4ca1-9843-f557c7ff77b0">




## @lionel- at 2023-07-07T08:24:47Z

Should we create a trait that allows us to call `send_or_warn()` or `send_or_error()` methods on channels? All this logging is a bit verbose. IIUC we can implement traits on foreign types as long as we own the trait.

## @DavisVaughan at 2023-07-07T15:24:26Z

@lionel- what do you think about https://github.com/posit-dev/amalthea/pull/62/commits/f9adfd6b7e2937eeb38faf3c1dc327d234c2f1e6?

It adds a `ResultOrLog` trait that is implemented for `Result<(), E>` in particular. We generate this kind of result object a lot, where we don't have any value but _may_ have some kind of error. This gives us the ability to chain on `.or_log_*()` to log the problem if one occurred.

It takes a `prefix: &str` because I think most of the time we will supply a simple string literal. If you want to interpolate the prefix (which we happen to do here) then you need to use `&format!()` since `format!()` returns a `String`. I can't think of anything better.

The name is inspired by the `.or()` and `.or_else()` methods that are already on `Result`s

## @lionel- at 2023-07-07T15:40:33Z

I like it!