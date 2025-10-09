# Use common interface to iterate over environments

> <https://github.com/posit-dev/ark/pull/538>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

Addresses: https://github.com/posit-dev/positron/issues/4700

also addresses crashes from:

- https://github.com/posit-dev/positron/issues/4686
- https://github.com/posit-dev/positron/issues/4741

The approach taken here was to parse the SEXPREC header and check if the `extra` field is set. `extra` being set would actually indicate that it's an immediate binding.


## @lionel- at 2024-09-20T12:52:31Z

@dfalbel We were planning on removing the dependency on sxpinfo to avoid Ark crashes when internals change. @DavisVaughan would like to go through with that change now: https://github.com/posit-dev/ark/pull/540/files

Could you please rewrite environment traversal so that it uses exposed API? E.g. `findVar()` (gets you values and promises), `R_BindingIsActive()`, `R_ActiveBindingFunction()`, etc.

A tricky part is that it has to work the same way on all R versions.

## @lionel- at 2024-09-20T12:59:45Z

@DavisVaughan Agreed, I had no idea we were this close. I guess our new environment iterator implementation went a long way.

I think the right way to approach things here is to create a new environment iterator that provides the binding type along the lines of https://gist.github.com/lionel-/1ebcbd5ec69c0775d514c329522408a3#binding-type. Then you could access this typed binding with specific accessors that we can tweak so they work well on all R versions.

## @dfalbel at 2024-09-23T17:59:02Z

I updated the PR to use `harp`'s internal environment iterator, which should safely expand immediate bindings when necessary,and correctly avoids the evaluation of promises and active bindings. We'll no longer account the size of internal implementation details of environments, such as the pre-allocated size of the hash table, or the size each chain uses, etc. 