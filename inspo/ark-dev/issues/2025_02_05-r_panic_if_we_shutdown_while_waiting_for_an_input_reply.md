# R: Panic if we shutdown while waiting for an `input_reply`

> <https://github.com/posit-dev/ark/issues/694>
>
> * Author: @DavisVaughan
> * State: OPEN
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABSPDwBw", name = "bug", description = "Something isn't working", color = "E99695")

Reprex:
- `menu(c("a", "b"))`
- Shut down the session

We get a structured error message that suggests we were not expecting this to ever occur, but maybe we should special case whatever is happening here to allow the shutdown.

It puts us in a state where we can't get into a new R session without a full shutdown of Positron, which is not great.


https://github.com/posit-dev/ark/assets/19150088/2070e5e0-efd5-4a9c-b84c-c3c86c13d1ef



