# Provide a stub for `YesNoCancel` for now, just to avoid a crash

> <https://github.com/posit-dev/ark/pull/186>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Shutting down on Windows was causing a crash because it asked `Save workspace image?` through `R_YesNoCancel()`, but I hadn't provided a hook for that yet.

I don't think we will actually end up using `R_YesNoCancel()` in the end. It is only called through R's default hook for `R_CleanUp()` when the save action is set to `SA_SAVEASK`. I think we are going to provide our own hook for `R_CleanUp()`, resolve `SA_SAVEASK` to a different save action using our own tooling, and then hand off to R's default cleanup hook after doing any other clean up we need on our end. (This is like what RStudio does).

If we did that, then `R_YesNoCancel()` should never actually get called so we could make it an error after that.

