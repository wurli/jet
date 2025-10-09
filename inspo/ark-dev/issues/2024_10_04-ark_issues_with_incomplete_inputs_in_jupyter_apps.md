# Ark: Issues with incomplete inputs in Jupyter apps

> <https://github.com/posit-dev/ark/issues/557>
> 
> * Author: @lionel-
> * State: CLOSED
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABSPDwBw", name = "bug", description = "Something isn't working", color = "E99695"), list(id = "LA_kwDOJkuGPc8AAAABwXx3RQ", name = "area: jupyter kernel", description = "", color = "C2E0C6")

When an incomplete prompt is detected and R needs more input, we reply with a `ExecuteResponse::ReplyException`. This should never be the case in Positron since we only send complete inputs, but this error is seen in Jupyter apps. There are currently a couple of issues:

- We see a weird syntax error caused by the way we wrap inputs in braces to avoid printing intermediate expressions (ignore the `INFO` message):
<img width="953" alt="Screenshot 2024-09-19 at 13 09 22" src="https://github.com/user-attachments/assets/0327414e-ce65-457c-870d-3a58e0fc2552">


- Once we fix this syntax error I'm pretty sure Ark will be in an unexpected state because we don't reset the REPL state and R is still waiting for more input to complete the current command. To fix this, we could try running `invokeRestart("abort")` (I'd rather not send an interrupt).

## @DavisVaughan at 2024-09-19T12:21:35Z

I do see that incomplete is also an error on the python side

<img width="379" alt="Screenshot 2024-09-19 at 8 20 56 AM" src="https://github.com/user-attachments/assets/32f9adb7-1c12-4abf-9e16-887dc8b45b4a">
