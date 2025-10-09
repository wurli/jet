# Add tool for querying multiple installed package versions

> <https://github.com/posit-dev/ark/pull/871>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

I noticed that the LLM sometimes makes several calls to `Get Installed R Package Version` in a row. To save time & tokens, this adds a new RPC that can look up multiple package versions in one call. 

The Positron side of this change will be included in an upcoming PR.

## @jennybc at 2025-07-12T17:15:05Z

We have existing package version tooling in `crates/ark/src/modules/positron/package.R`. I don't know if there's an argument for keeping everything exposed via tools in one place and self-contained, but thought I'd remind us of this other file:

https://github.com/posit-dev/ark/blob/main/crates/ark/src/modules/positron/package.R

For example, those functions take care to not (necessarily) load the package, if we care about that.

## @jmcphers at 2025-07-15T23:22:40Z

@jennybc thank you for pointing that out! I'm putting this in a separate file b/c it is specifically for use by the LLM (the other script is called by other frontend functions). Stuff we want the LLM to use is going to have different semantics, e.g. here we have `Not installed` for a package that is not installed instead of something more composable like `NULL`. 

I don't feel strongly that these live in their own file; I could also put them in the main package script and annotate them re: their LLM use. Let me know if that seems better?

## @jmcphers at 2025-07-15T23:24:41Z

@lionel- you've made this much better, thank you! The only thing I ran into when testing the new approach is that it does not work for the base package (which the LLM likes to check the version of as a way of checking R's version sometimes...).

```
> getNamespaceInfo(asNamespace("base"))
Error in `asNamespace()`:
! operation not allowed on base namespace
Hide Traceback
    ▆
 1. └─base::getNamespaceInfo(asNamespace("base"))
 2.   └─base::asNamespace(ns, base.OK = FALSE)
 3.
```

 I've made a small change to fall back to regular libPath searching for the base package.