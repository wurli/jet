# Rewrite `on_process_idle_events()` as an async idle task

> <https://github.com/posit-dev/ark/issues/791>
> 
> * Author: @lionel-
> * State: OPEN
> * Labels: 

Right now plot requests are handled at idle time when no user executions are running in the console:

https://github.com/posit-dev/ark/blob/40908a3b2ba4c54bba05940ffd536444abf598a0/crates/ark/src/interface.rs#L842

The current way we poll for the messages is timeout-based, with a freq of 200ms at the time of writing. This approach translates to visible delays in the frontend when a plot needs to be rerendered. I'm about to reduce the frequency to 50ms but ideally we wouldn't wait at all when a request is ready.

To fix this, we could spawn an async idle task with `r_task::spawn_idle()` that runs an infinite loop selecting on the plot comm channels. This would involve:

- Switching to tokio async channels instead of sync crossbeam channels.
- Setting things up so that `is_drawing` and `should_render` causes the task to wait until the graphics device is ready to render.

This approach will improve overall performance of the communication between the frontend and Ark for plots (part of https://github.com/posit-dev/positron/issues/5184).


## @lionel- at 2025-06-12T14:57:48Z

Will help with things like https://github.com/posit-dev/ark/pull/836#issuecomment-2966530874

## @lionel- at 2025-06-12T15:41:15Z

oops sorry, this particular issue is only about plots.

For other polled events we have to rely on sampling.