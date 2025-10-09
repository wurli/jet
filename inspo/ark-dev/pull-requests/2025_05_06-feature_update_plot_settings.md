# Handle notifications for updated plot render settings

> <https://github.com/posit-dev/ark/pull/792>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Ark side of https://github.com/posit-dev/positron/pull/7450.
Progress towards posit-dev/positron#7449.

The goal of this PR is to handle the new UI comm notification `DidChangePlotRenderSettings` to update the settings used by the graphics device for pre-renderings of plots. This replaces the use of `render` request from which we currently pluck render settings and use that for next renderings. This change brings several improvements:

- We receive the new notification more often: on startup, when the sidebar/panel size changes, when the sizing policy changes, etc.

- Some render requests do not reflect an actual change in render settings. For instance if the user saves a plot, this sends a render request that should not update the current render settings for the next new plot.

The tricky part of this PR was to get the graphics device to listen to the new notification. Since the device owns the current render settings state and since it runs on the R thread, it was not possible to update the settings from an auxiliary thread in charge of watching the notifications. I realised that instead of an auxiliary thread, it would be ideal to give that responsibility to a _cooperative task_ that runs at interrupt time and yields back to the R thread after each message handling. The best interface for such async cooperation would be an async future polled by an executor running on the R thread.

Fortunately we already had most of the infrastructure in place for spawning future-based tasks on the R thread. We mostly use it for _idle_ tasks, but we also support _interrupt_ tasks which we use for dropping `RObject`s owned by other threads. This leads to the following pattern which should be useful in other cases too:

```rust
async fn process_notifications(
    mut graphics_device_rx: AsyncUnboundedReceiver<GraphicsDeviceNotification>,
) {
    loop {
        while let Some(notification) = graphics_device_rx.recv().await {
            match notification {
                GraphicsDeviceNotification::DidChangePlotRenderSettings(plot_render_settings) => {
                    // Handling
                },
            }
        }
    }
}

// Called during init
r_task::spawn_interrupt(|| async move { process_notifications(graphics_device_rx).await });
```

The task is an infinite loop that is woken up by new notifications. Once woken up, the R thread yields to the task at the next interrupt check. The R thread is yielded control back at the next attempt to read a notification.

Since this task is spawned very early during startup I had to make some changes to how we initiate the task state to prevent a deadlock. I think this simplifies our setup and removes the unsightly timeout wait when a task is spawned too early. Instead the tasks are accumulated and blocked from running until after startup.

Another change concerns how we run polled events at idle time. We're currently using timeout-based polling because we have no way of getting woken up by R events. However the way we set that up was causing the timeout to be reset everytime some other event woke up our event loop. To prevent that, the timeout is now a tick channel that is selected on, which ensures it runs regularly.

I've also reduced the timeout from 200ms to 50ms. The larger timeout caused a visible delay in plot rerenderings (for the remaining cases where a rerender is necessary, i.e. after the history strip appears in the plot pane, causing the plot area to shrink). We could improve performance further with the same sort of async task as described above (except running at idle time). I wrote about this in https://github.com/posit-dev/ark/issues/791.

In this PR you'll also see some noise:

- The plot types were renamed for consistency.
- I had to move some tests from integration to unit to account for a tightening of interface visibility.


