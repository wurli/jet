# Use `startup_file` to force a standardized `repos` option during testing

> <https://github.com/posit-dev/ark/pull/650>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Because https://github.com/r-lib/rig/issues/203 forces a non-`@CRAN@` default for my `CRAN` repo, meaning that the tests don't pass for me locally because `apply_repo_defaults()` sees that something is set and doesn't override it.

