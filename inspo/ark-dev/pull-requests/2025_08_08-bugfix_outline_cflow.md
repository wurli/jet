# Recurse into special constructs to find document symbols

> <https://github.com/posit-dev/ark/pull/892>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/8881

We now consistently recurse inside these constructs to collect nested document symbols such as comment sections:

- if-else branches
- loops

The behaviour regarding nested assignments implemented in https://github.com/posit-dev/ark/pull/859 has been adapted so that `{` blocks in if-else branches and loops still counts as top-level. This way these assignments are all treated as consistently part of the outline:

```r
x <- 1

if (TRUE) {
  x <- 2
} else {
  x <- 3
}

for (i in xs) {
  x <- 4
}
```

In addition we also now collect comment sections in parameter lists of function definitions:

```r
x <- function(
  # section ----
  arg
) {
  ...
}
```

### QA Notes

Comment sections and other kinds of document symbols in if-else branches and loops are now part of the outline:

```r
# top ----

if (TRUE) {
  # if ----
  foo <- function() {}
} else {
  # else ----
  foo <- function() {}
}

for (i in xs) {
  # loop ----
  bar <- function() {}
}
```

Comment sections in parameter lists of function definitions are now included as well. We don't recurse through default arguments though.

```r
x <- function(
  # section ----
  arg = function() {
    # not a section ----
  }
) {
  ...
}
```

<img width="857" height="601" alt="Screenshot 2025-08-08 at 13 54 15" src="https://github.com/user-attachments/assets/919cfc00-c1fb-4c3b-bdf5-8c2f99bbb15e" />


