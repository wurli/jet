# Restore `top_level` state on exit

> <https://github.com/posit-dev/ark/pull/879>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Bug revealed by E2E Positron tests: I was not restoring the top-level state after leaving a nested state. So top-level variables appearing _after_ a context with nested blocks were not shown in the outline.

This is now also tested on the backend-side.

cc @DavisVaughan 

