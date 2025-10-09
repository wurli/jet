# Downgrade problems in `harp_format()` to trace

> <https://github.com/posit-dev/ark/pull/483>
> 
> * Author: @juliasilge
> * State: MERGED
> * Labels: 

Closes #479 

Addresses https://github.com/posit-dev/positron/issues/3115

Today we are again seeing examples of these errors/warnings showing up in logs as red herrings: https://github.com/posit-dev/positron/discussions/4437#discussioncomment-10433103 so I do think we at least want to fix up the logs.

This does _not_ actually fix how we are displaying formulas and other objects like quosures which have length > 1 but `base::format()` results of length 1, i.e. does _not_ address https://github.com/posit-dev/positron/issues/4119.

I also changed the name of the functions to more clearly specify they are about vectors.

