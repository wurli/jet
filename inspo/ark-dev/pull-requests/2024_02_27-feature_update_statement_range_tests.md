# Update statement range tests for new tree-sitter-r

> <https://github.com/posit-dev/ark/pull/252>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/1464

We finally figured out how to fix this issue upstream in tree-sitter-r through
https://github.com/r-lib/tree-sitter-r/pull/72

Here is a recording of the reprex from https://github.com/posit-dev/positron/issues/1464#issuecomment-1787666141, working much better now. It still isn't quite right when we have two expressions on the same line, i.e. `}; lapply(1:5, print)`, but that is a separate bug related to us not taking the `column` into account, I think. I put that in https://github.com/posit-dev/ark/issues/714

https://github.com/posit-dev/amalthea/assets/19150088/07d58dbb-62e1-4007-951b-3ae1d8a92f40



