# Avoid providing composite completions after `::` when the package isn't installed

> <https://github.com/posit-dev/ark/pull/834>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Closes #833 

Instead, provide no completions

So `styler::sty` shows no completions if you don't have styler installed, rather than random session completions.

