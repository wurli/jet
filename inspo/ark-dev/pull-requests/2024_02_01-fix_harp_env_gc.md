# Fix `HARP_ENV` garbage collection

> <https://github.com/posit-dev/ark/pull/230>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/2188

Accidentally introduced in https://github.com/posit-dev/amalthea/pull/223. @lionel- do you agree with the changes I made here? (Going to go ahead and merge since it seems to fix the crashes we are seeing).

`HARP_ENV` is different from the other positron/rstudio envs, because those other envs are temporary and get copied into `tools:positron` and `tools:rstudio` in calls like this: `export(exprs, from = ns, to = as.environment("tools:positron"))`. We need `HARP_ENV` to be persistent past the `RObject`'s `Drop` call.

