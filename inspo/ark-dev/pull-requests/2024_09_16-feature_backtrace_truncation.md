# Truncate error calls and backtraces

> <https://github.com/posit-dev/ark/pull/524>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

The logs in https://github.com/posit-dev/positron/issues/4686 can't be read because the error calls and backtraces include 1000s of lines of inlined data.

To prevent this, we now truncate error calls to ~10~ 50 lines and R backtraces to 500 lines.

