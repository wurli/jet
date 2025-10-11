# Fix `Sys.getenv()` completions, support `Sys.unsetenv()` too

> <https://github.com/posit-dev/ark/pull/530>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/4677

I think I accidentally broke this with https://github.com/posit-dev/ark/pull/439 because I didn't know we were relying on the argument labels _only_ containing the argument name (they now also contain the default value where applicable).

I _feel_ like the LSP `SignatureHelp` data structure is probably not completely appropriate for what we want here since it is used for UI purposes, and we need something that is still useful for static analysis in other places besides the LSP request handler. But I'm not ready to shave that yak right now.

I've added lots of tests for all the custom completion types we support, so we can't accidentally break this in the future.

