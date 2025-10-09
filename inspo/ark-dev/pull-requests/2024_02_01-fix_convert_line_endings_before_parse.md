# Convert `\r\n` to `\n` before calling `parse(text =)`

> <https://github.com/posit-dev/ark/pull/229>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Fixes a buglet with the new module loading procedure on Windows. Same issue as `R_ParseVector()` where the text can't have `\r\n` in it, even on Windows. Luckily we have the tooling for that.

