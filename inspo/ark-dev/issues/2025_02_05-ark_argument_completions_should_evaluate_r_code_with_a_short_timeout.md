# Ark: Argument completions should evaluate R code with a short timeout

> <https://github.com/posit-dev/ark/issues/688>
>
> * Author: @lionel-
> * State: CLOSED
> * Labels:

- Wrap `ParseEval()` with a variant that quickly and orderly times out to avoid freezing R and the LSP.

- Review all `RFunction::call()` calls to do the same, for instance the one that evaluates `names()` (a generic that might dispatch to foreign code) on an arbitrary object may potentially freeze the R event loop too.

This will require the mechanism discussed in https://github.com/posit-dev/positron/issues/1419#issuecomment-1810222809.

## @lionel- at 2025-02-05T21:38:51Z

Would be nice but not a priority currently
