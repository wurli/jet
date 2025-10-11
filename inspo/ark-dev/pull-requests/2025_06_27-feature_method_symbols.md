# Emit functions passed as named arguments as document symbols

> <https://github.com/posit-dev/ark/pull/858>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

Branched from #856
Addresses https://github.com/posit-dev/positron/issues/6546
Addresses https://github.com/posit-dev/positron/issues/6107 and https://github.com/posit-dev/positron/issues/5241

The main functionality this PR aims to implement is inclusion of R6 symbols in the outline.

```r
class <- R6::R6Class(
  'class',
  public = list(
    initialize = function() 'initialize',
    foo = function() 'foo'
  ),
  private = list(
    bar = function() 'bar'
  )
)
```

I think it makes sense to include _all_ functions passed as named arguments, to any calls and not just `R6Class`. This is in contrast to _workspace symbols_ reachable across files. In this case I think only R6 methods should be registered as workspace symbols (see https://github.com/posit-dev/positron/issues/6549). In general document symbols are more exhaustive than workspace one, offering access to local objects.


Changes:

- We now collect symbols in the RHS of assigned objects. In particular this means we get a chance to collect methods in the `r6class()` call above. This also means we'll collect symbols in cases like this, which I think makes sense:

  ```r
  foo <- {
    bar <- function() {}
  }
  ```

- We now collect symbols across call arguments. This allows us to collect methods nested deeply inside the `r6class()` call. This also means we also collect symbols in cases like the following, which I also think makes sense:

  ```r
  local({
    a <- function() {
      1
    }
  })
  ```

  Addresses https://github.com/posit-dev/positron/issues/6107 and https://github.com/posit-dev/positron/issues/5241

- Finally, functions passed as named arguments are collected as Method symbols. Previously only functions assigned with `<-` at top level or in `{}` would be collected.


### QA Notes:

All these changes are tested in the backend. On the frontend you should now see the outline/breadcrumbs filled with `initialize`, `foo`, and `bar` methods.

```r
# section ----
class <- R6::R6Class(
  'class',
  public = list(
    initialize = function() 'initialize',
    foo = function() {
      1
      # section2 ----
      nested <- function() {}
      2
    }
  ),
  private = list(
    bar = function() {
      3
    }
  )
)

list(
  foo = function() {
    1
  }, # matched
  function() {
    nested <- function () {
        # `nested` is a symbol even if the unnamed method is not
    }
  }, # not matched
  bar = function() {
    3
  }, # matched
  baz = (function() {
    4
  }) # not matched
)

local({
  a <- function() {
    1
  }
})
```


https://github.com/user-attachments/assets/c369c5bc-2506-482b-a3f2-fa1a8146ba58



## @lionel- at 2025-07-03T14:32:06Z

> Is there a reason the changes in this PR are tested only that way? Versus the more concrete assertions in some of the other tests. I'm definitely thinking about my recent foray into completions and tests, where it was possible to check that a certain completion was present (and of a specific form) or absent.

It's just for convenience, of both writing the tests and updating them (i.e. if you insert a line all ranges change).
