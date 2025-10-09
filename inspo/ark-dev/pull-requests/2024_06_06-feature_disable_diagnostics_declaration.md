# Disable diagnostics in generated namespaces

> <https://github.com/posit-dev/ark/pull/386>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses posit-dev/positron#3320.

This adds support for top-level declarations in a file using the new `declare()` primitive added in R 4.4.0. For backward compatibility we accept declarations wrapped in a `~`.

The only supported declaration is `diagnostics = FALSE` to turn off diagnostics in this file. This is used to turn off diagnostics in generated namespace files from https://github.com/posit-dev/amalthea/pull/251.

### Syntax

Edit: We went for option C.

We can always change the syntax later on so we don't need to nail it right away but here are some considerations. The declaration must be namespaced within `ark()`, e.g.:

```r
declare(ark(diagnostics = FALSE))
```

In the future this could be extended like this:

```r
# Option A

declare(ark(
  diagnostics(lintSomething = FALSE)
))

# Easily disable without changing config
declare(ark(
  diagnostics = FALSE,
  diagnostics(lintSomething = FALSE)
))
```

This sort of `declare()` annotations was discussed at the R sprint. The idea is that a "function call" is the syntax for a named list. This way we don't need to use `c()` or `list()` to create a list, which would be unsatisfactory because these expressions are not evaluated and should be pure syntax.

Here is an alternative to `diagnostics = ` which might be better because all diagnostics related options are gathered in the same list:

```r
# Option B

declare(ark(
  diagnostics(enable = FALSE)
))

declare(ark(
  diagnostics(enable = FALSE, lintSomething = FALSE)
))
```

And here is an alternative for ark namespacing:

```r
# Option C

declare(
  ark::diagnostics(enable = FALSE)
)

declare(
  ark::diagnostics(enable = FALSE, lintSomething = FALSE)
)
```

Well actually I might prefer this last one. But it might have lots of repeated `ark::` as we add new named lists of annotations.

