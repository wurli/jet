# Rename `CompletionContext` to `DocumentContext`

> <https://github.com/posit-dev/ark/pull/142>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

`CompletionContext` was a fairly generic object containing:
- A document
- The source of that document as a string
- A point in that document
- A node corresponding to the point

This makes it agnostic to completions, and indeed we use this same type in `hover()`, so I've renamed it to `DocumentContext` and extracted it out into its own file

