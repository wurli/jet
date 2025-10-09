# Detect assignments in calls to avoid false positive lints

> <https://github.com/posit-dev/ark/pull/639>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses posit-dev/positron#3048.

## Positron Notes

### Release Notes

#### New Features

- N/A

#### Bug Fixes

- Assignments in function calls (e.g. `list(x <- 1)`) are now detected by the missing symbol linter to avoid annoying false positive diagnostics (posit-dev/positron#3048). The downside is that this causes false negatives when the assignment happens in a call with local scope, e.g. in `local()` or `test_that()`. In these cases the nested assignments will incorrectly overlast the end of the call. We prefer to be overly permissive than overly cautious in these matters.

### QA Notes

Assignments in calls, e.g. `list(x <- 1)` should now be treated the same as at top-level. Any further references to `x` at top level should not be linted, e.g. `y` should cause a lint but not `x` in:

```r
list(x <- 1)
x; y
```

Note that calls can be nested `list(x <- 1, list(y, x, y <- 2), 1, y)`.

In that example, the first `y` should in principle be linted as it hasn't been defined yet but we don't support this sort of lints inside arguments yet.


