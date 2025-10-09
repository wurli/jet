# Protect bindings from garbage collection

> <https://github.com/posit-dev/ark/pull/152>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/1812

Since they are moved to the calling thread under the protection of an `RThreadSafe` wrapper that prevents dropping on the wrong thread, all that is needed is to store the bindings as `RObject` instead of `SEXP`.

I've added an `Eq` implementations for `RObject` in the process as it was required to fulfill `derive(PartialEq)` on `BindingValue`.

