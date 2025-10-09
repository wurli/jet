# Refactor prompt detection

> <https://github.com/posit-dev/ark/pull/51>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Progress towards rstudio/positron#535.

To solve readline interrupts we'll need to detect readline prompt from `r_read_console()`. This PR creates a new `PromptInfo` struct that contains all information about the prompt type (user request? incomplete code?). This information is created synchronously in `r_read_console()` before being sent away to other threads. Previously the prompt type was detected from the ark-execution thread by querying R state without taking the R lock which is unsafe.

The PR also improves the way prompt types are detected. Currently the prompt string is compared to `getOption("prompt")` and `getOption("continue")`. This could be fooled by e.g. `readline("> ")` or `readline("+ ")`. Instead, we now call `sys.nframe()` to detect whether we are at top-level. If that's the case, then we can compare the prompt to `getOption("continue")` safely.

In a future PR for rstudio/positron#407 we'll also detect browser prompts using the equivalent of `rlang::env_is_browsed(sys.frame(sys.nframe()))`, i.e. inspecting the last frame on the stack to see if it has been marked for debugging. If a browser prompt, we'll consider that it can't be a readline prompt. There are very rare edge cases where this might not be true, e.g. `debug(readline)`, but I don't think we can do better without getting this information from R itself in an implove `ReadConsole()` callback.

