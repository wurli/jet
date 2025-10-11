# Return NULL if context is NULL

> <https://github.com/posit-dev/ark/pull/237>
>
> * Author: @jennybc
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/2191

This is the most basic solution to this problem, which is to return `NULL` for `rstudioapi::getSourceEditorContext()` (and, therefore, for now, `rstudioapi::getActiveDocumentContext()`), when no document is open. This is clearly correct for `getSourceEditorContext()`. It's not so correct for `getActiveDocumentContext()`, which currently is effectively aliased to `rstudioapi::getSourceEditorContext()`:

https://github.com/posit-dev/amalthea/blob/33351343592c1cdbee3b3d923ef923a862208088/crates/ark/src/modules/rstudio/document-api.R#L2-L4

The correct solution, which requires more work, is to properly acknowledge the Console as an editor. That's necessary to exactly match the RStudio behaviour seen in https://github.com/posit-dev/positron/issues/2191. I'm not doing that here and I think it's not super urgent, but we can add it to the list in https://github.com/posit-dev/positron/issues/1312.

Here's behaviour with this PR:

https://github.com/posit-dev/amalthea/assets/599454/115ae61e-ae88-4d04-8a10-4cfae25bedea





