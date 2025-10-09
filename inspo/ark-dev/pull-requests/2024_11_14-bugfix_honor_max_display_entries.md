# Honor `MAX_DISPLAY_ENTRIES` when inspecting variables

> <https://github.com/posit-dev/ark/pull/629>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

This PR fixes some performance related issues that involve the variables pane.
Addressing: 
- https://github.com/posit-dev/positron/issues/4636 
- https://github.com/posit-dev/positron/issues/4573
- https://github.com/posit-dev/positron/issues/2223
- https://github.com/posit-dev/positron/issues/3628

The goal is to have the following scenarios working fine in positron:

```r
# create a single string that's very large
a <- paste0(rep(letters, length.out = 1e7), collapse="-")

# create a very large vector, and expand it in the variables pane
b <- 1:100000

# expand a data frame with many columns
d <- as.data.frame(lapply(1:10000, function(x) 1:100000))

# expand a data.frame with many rows
f <- as.data.frame(lapply(1:20, function(x) 1:5e6))

# visualizing large matrixes (many rows and columns)
g <- matrix(0, nrow = 100000, ncol = 10000)

# matrixes with large strings inside
h <- matrix(paste0(rep(letters, length.out = 1e7), collapse="-"), nrow = 10, ncol = 10)
```

Note: When testing it's important to use a Release version of ark. In my tests, with debug can be 20x slower in such cases - probably for the tail recursion optimizations important for the object size computation.

Not fixed in this PR: 

- https://github.com/posit-dev/positron/issues/2894
- If a package extends the variables pane and it's methods are slow
- R6 classes with many methods are not truncated yet
- S4 objects with many slots are not truncated
- Still expanding the `model` from https://github.com/posit-dev/positron/issues/4636 takes about 1s. I *think* a lot of this time is computing the objects size - because of the deeply nested object.
- https://github.com/posit-dev/positron/issues/3656 (This should be a quick fix as we are here)


