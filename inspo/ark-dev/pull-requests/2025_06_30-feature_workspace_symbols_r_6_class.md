# Emit R6Class methods as workspace symbols

> <https://github.com/posit-dev/ark/pull/861>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/6549

- Add a bunch of tests for the workspace indexers which were not tested before.

- Add a new indexed type `Method`. This is used to include R6Class methods in workspace symbols but exclusing them from completions.

- Indexers now push symbols onto a list instead of returning a single symbol to their callers. This allows a single indexer to handle multiple symbols.

- The assignment handler now pushes R6Class method symbols in addition to the assigned object.


### QA Notes

Add the following to a file:

```r
class <- R6::R6Class(
    'class',
    public = list(
      initialize = function() 'initialize',
      foo = function() {
        1
      }
    ),
    private = list(
      bar = function() {
        2
      },
      not_indexed1 = NULL
    ),
    other = list(
      not_indexed2 = function() {
        3
      }
    )
  )
```

These symbols should now be available as Workspace symbols (`#` prefix in command palette): `initialize()`, `foo()`, `bar()` and you should be able to jump straight to the definition of these methods from any file.

The symbols starting with `not_indexed` should not be available.

All this is tested on the backend side.


