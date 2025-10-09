# Reticulate support

> <https://github.com/posit-dev/ark/pull/506>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

Add support for reticulate sessions. See https://github.com/posit-dev/positron/pull/4603

## @lionel- at 2024-09-26T07:59:27Z

My main comment is about `try_lock()` which should be replaced by `lock()` to avoid rarely occurring surprising behaviour.

## @dfalbel at 2024-09-26T15:18:34Z

I addressed most points in https://github.com/posit-dev/ark/pull/506/commits/286a24aa0000ba736694dd133e9de117ba9d7d20

Also moved `.ps.rpc.install_reticulate()` into a more general `.ps.rpc.install_packages`.