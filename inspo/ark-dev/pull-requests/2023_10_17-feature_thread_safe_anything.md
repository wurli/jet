# `RThreadSafeObject` -> `RThreadSafe<T>`

> <https://github.com/posit-dev/ark/pull/114>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Part of https://github.com/posit-dev/positron/issues/1550

Expansion of #111 with @lionel- that allows us to wrap "anything" (within reason) in `RThreadSafe<T>`, rather than just `RObject`s (like `CharacterVector`s).

This helps with `Binding`s, as we can wrap the `Vec<Binding>` in `RThreadSafe<>` so that when the vector is dropped we ensure that that happens on the main R thread. This isn't _strictly_ necessary right now, as a `Binding` is made up of two parts:
- A `RSymbol`, which is a `SEXP` assumed to be a `SYMSXP` that is not protected by anything
- A `BindingValue`, which is an enum that contains one or more `SEXP`s that represent the value of a binding in an environment. This isn't protected in any way either, because we assume the environment does it.

Because `Binding` doesn't protect anything, it doesn't implement `Drop` so we don't really need to worry about it getting dropped on the wrong thread.

However, it does need to be able to send the underlying `SEXP` objects between threads, so this is what we are really using `RThreadSafe` for right now.

In the future, if we update `Binding` to also do some kind of protection, then we should be in a good place to ensure that the drops happen on the main R thread.



