# Define include, share, and doc environment variables

> <https://github.com/posit-dev/ark/pull/640>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/3637.


## Positron Notes

### Release Notes

#### New Features

- N/A

#### Bug Fixes

- The following environment variables are now set in the same way that R does:

  - `R_SHARE_DIR`
  - `R_INCLUDE_DIR`
  - `R_DOC_DIR`

  This solves a number of problems in situations that depend on these variables being defined (https://github.com/posit-dev/positron/issues/3637).

### QA Notes

This should not fail:

```r
stopifnot(
  nzchar(Sys.getenv('R_SHARE_DIR')),
  nzchar(Sys.getenv('R_INCLUDE_DIR')),
  nzchar(Sys.getenv('R_DOC_DIR'))
)
```

We test for these on our side.

