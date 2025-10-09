# Preserve focus when user evaluates expression during debug session

> <https://github.com/posit-dev/ark/pull/796>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/3151

## @lionel- at 2025-05-13T11:48:35Z

@DavisVaughan I considered where to store that state and to me `RMain` felt like the right place because whether to focus or not is all about whether we're entering a nested debug repl. In the ideal setup, if we had our own repl, that state would be communicated by ReadConsole. The fact that the DAP can _in the current approach_ figure it out on its own by comparing the call stack is an implementation detail that shouldn't affect the proper boundaries of concerns. What do you think? Happy to discuss over a call.

## @DavisVaughan at 2025-05-13T13:05:59Z

We decided to keep the current code where it is to match how we think about `preserve_focus` being an input to the DAP tooling, rather than something it computes itself

## @DavisVaughan at 2025-05-13T15:08:56Z

@lionel- did you want to look further at the issue in the video though? https://github.com/posit-dev/ark/pull/796#pullrequestreview-2834259832