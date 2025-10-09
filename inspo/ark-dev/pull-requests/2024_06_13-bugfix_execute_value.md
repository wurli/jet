# Fixes for Jupyter compatibility

> <https://github.com/posit-dev/ark/pull/405>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/281
Addresses https://github.com/posit-dev/positron/issues/786

Follow-up to https://github.com/posit-dev/amalthea/pull/65

- We now use a set of heuristics to detect output emitted during top-level autoprint. This output is accumulated and reemitted in `execute_result` messages on IOPub. This detection fails in some cases, most importantly browser prompts. In the longer term, we really need an extended `WriteConsoleExt()` method with which R can announce whether an output is emitted during autoprint.

- In Notebook mode, we now wrap inputs in braces so that `1; 2; 3` only includes `3` as part of execution results.

- In Notebook mode, we now include error's `evalue` field in the `traceback` field. This is necessary because Jupyter frontends typically ignore `evalue`. We may want to do this by default and have Positron detect redundant mentions of errors by comparing `evalue` and `traceback[0]`, this way we'll be generating conventional traceback fields in Console mode while still being able to disambiguate error messages and backtraces.

- To support the special notebook behaviour, `SessionMode` is now propagated to `RMain`. The kernel spec created with `--install` now sets the mode to notebook so that Jupyter apps benefit from these fixes.

- Various cleanups and refactoring to make this part of the code cleaner.

Together with #400, these changes should make Ark in pretty good shape regarding Jupyter compatibility.

