# Add `r_check_stack()` and use it to postpone R tasks

> <https://github.com/posit-dev/ark/pull/148>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses posit-dev/positron#1817

## @lionel- at 2023-11-15T17:33:49Z

@DavisVaughan The crash I mentioned in our call was because of a recursive call to `r_check_stack()` because `RFunction::call()` (used to reset the error buffer) evaluates R code which might trigger polled events which then checks the stack again etc. I've now disabled polled events from polled events more comprehensively.

I let the inner sandbox in place as a defensive measure, though we could now remove it since polled events are disabled in the two places we call yield-to-tasks.

## @lionel- at 2023-11-16T11:19:14Z

> I let the inner sandbox in place as a defensive measure, though we could now remove it since polled events are disabled in the two places we call yield-to-tasks.

It was a bit wasteful to run `polled_events()` in a top-level context so I pulled the parts of `r_sandbox()` that were of interest (interrupt and polled events suspension) into `RSandboxScope` on the model of `RInterruptsSuspendedScope`. This is now used in `polled_events()` instead of `r_sandbox()`.