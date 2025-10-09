# Some fixes to make `edit()` and `file.edit()` more robust

> <https://github.com/posit-dev/ark/pull/68>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/rstudio/positron/issues/856
Addresses part of https://github.com/rstudio/positron/issues/857 (we can open >1 file now, but `enablePreview` still messes with it)

Various improvements to make editing files through `edit()` and `file.edit()` more robust, I'll point them out inline

@kevinushey do we have any testing infra for the public/private modules? It seems like it would be nice to add some kinds of tests for the error cases here at least, but I'm not sure where to put them or how they'd get run 😢 

