# Implement Positron functionality in separate modules

> <https://github.com/posit-dev/ark/issues/587>
> 
> * Author: @lionel-
> * State: OPEN
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABwXx3RQ", name = "area: jupyter kernel", description = "", color = "C2E0C6")

Currently `RMain` knows quite a lot about the Positron comms. For instance `RMain` has a channel to get events from the UI comm and send UI comm requests to the frontend. There are two main tasks to achieve that:

---

To make progress towards https://github.com/posit-dev/positron/issues/3608, we should remove all of that state and logic in separate modules with a clean boundary. The external components would then register handlers to be called at various points:

- Show message
- Busy
- Top-level refresh (run after each execute-request has completed)

The handlers would be called from the R thread, and so it would be safe for them to call the R API. `RMain` would allow for a list of handlers, meaning
that multiple external components could register handlers for `Busy`, for example.

They would also be passed some state:

- For example refresh handlers would be passed a reference to the prompt info.

- In the future they could receive a reference to the current structure of the global and debug environments. Ark would be in charge of creating this representation, stopping when it's too deeply nested, etc. All components (variables, LSP, DAP) would consume this representation.

---

One difficulty wrt the UI comm is `RMain::call_frontend_method()`. Currently, when an RStudio API method needs some information from the frontend, this sequence happens:

- .Call() into the Rust side
- The Rust side calls `RMain::get()` to get the global singleton
- Then `RMain::call_frontend_method()` is called.

Retrieving a global singleton in this way is necessary because R routines do not know anything about the current state of Ark. But the singleton doesn't need to be `RMain`. Ideally they would know nothing about `RMain`. So we could create a similar singleton for `UiComm` that would be set up when `UiComm::start()` is called.

One difficulty is that `call_frontend_method()` needs the currently active request's originator to be able to create an StdIn request. Since the active request live on `RMain`, we need to fix that first (https://github.com/posit-dev/ark/issues/586).


## @lionel- at 2024-10-15T15:17:03Z

The handler task would be helpful for https://github.com/posit-dev/positron/issues/5024 because it would be a nice way to switch to a "push" approach.

## @DavisVaughan at 2024-10-15T15:17:59Z

An outline of one of these handlers might look like

```rust
use crate::interface::PromptInfo;

pub struct KernelRefreshParams<'a> {
    pub prompt_info: &'a PromptInfo,
}

pub trait KernelHandler: Send {
    /// Handle top-level refresh.
    ///
    /// This is called after each top-level command. You can safely access the R
    /// API from this handler.
    #[allow(unused)]
    fn handle_refresh(&mut self, state: &KernelRefreshParams) -> anyhow::Result<()> {
        Ok(())
    }
}

```