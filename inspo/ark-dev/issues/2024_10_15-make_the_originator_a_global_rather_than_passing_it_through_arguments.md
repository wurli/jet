# Make the `Originator` a global rather than passing it through arguments

> <https://github.com/posit-dev/ark/issues/586>
> 
> * Author: @DavisVaughan
> * State: OPEN
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABwXx3RQ", name = "area: jupyter kernel", description = "", color = "C2E0C6")

- Can automatically be applied to "nested" stdin requests that happen within an execute request
  
- Can be applied to stdin requests that happen from `call_frontend_method()` when we move that method to the `UiComm`. Currently, having the originator as an argument makes this move difficult because we can't easily get the originator over to the `UiComm` (we could potentially ask `R_MAIN` for it, but that's a little gross).
  
- We already use a somewhat similar approach when we do IOPub busy/idle through having a "context" for each IOPub message

