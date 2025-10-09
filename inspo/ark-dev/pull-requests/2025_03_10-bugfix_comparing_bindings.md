# Improve bindings comparisons for the variables pane

> <https://github.com/posit-dev/ark/pull/737>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

This PR addresses: https://github.com/posit-dev/positron/issues/6618

The main issue is that when comparing two S4 connections objects (that include environments as attributes) we're using all the derived methods and ultimately resolving to `impl PartialEq for BindingNestedEnvironment` which always says `false` if objects have nested environments.

However, the variables pane is not trying to compare by value, but only for identity. 
This PR removes the `has_nested_environment` attribute that doesn't seem to be used anywhere and proposes that equality of Bindings are:

1. Their symbols are identical
2. Their values are identical (ie, they are pointing to the same `SEXP`)

We might want to use a different naming for this compariso, instead of overloading `eq`, let me know what you think.

## @dfalbel at 2025-03-12T10:42:54Z

Thanks! I have updated the PR to use `id()` as you suggested. In the variables pane we them compare binding values by `id()` when needed.

I've also dropped all derivations of `eq()` for `Bindings` and `BindingValues` to avoid confusions while we don't really need comparisions by value.

Let me know what you think

## @dfalbel at 2025-03-14T15:53:49Z

@lionel- I have updated the PR to avoid using the recursive type and now avoid the ambiguity with an enum of tuples of SEXP's.