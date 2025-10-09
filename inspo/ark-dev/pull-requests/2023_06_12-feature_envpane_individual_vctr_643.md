# Showing vector individual values in environment pane

> <https://github.com/posit-dev/ark/pull/31>
> 
> * Author: @romainfrancois
> * State: MERGED
> * Labels: 

Addresses https://github.com/rstudio/positron/issues/643

With 

```
x <- list(c(as.raw(1), as.raw(2)), c(FALSE, TRUE), c(1L, 2L), c(1, 2), c(0+1i, 0+2i), c("a", "b"), as.factor(c("a", "b")))
```

each element of `x` can expand to: 

<img width="573" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/537aee90-182e-4032-adbb-6506eabf0bf0">

## @romainfrancois at 2023-06-12T13:36:04Z

TODO: 
 - handle names, i.e. display them instead of `[1]` 
 - handle non trivial vectors: 

```r
y <- rep(Sys.time(), 2)
```

<img width="574" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/0f14ddca-f0a9-41ab-ab2e-41a2b1fdb8ef">


## @romainfrancois at 2023-06-13T07:23:27Z

Now handling names and types

```r
x <- list(c(a = as.raw(1), b = as.raw(2)), c(c = FALSE, d = TRUE), c(e = 1L, 2L), c(f = 1, 2), c(g = 0+1i, 0+2i), c(h = "a", "b"), as.factor(c(i = "a", "b")), rep(Sys.time(), 2) )
```

expanding to: 

![image](https://github.com/posit-dev/amalthea/assets/2625526/e905e0d7-6a40-4431-825d-ea6f62c0ece6)

I'm wondering if the display type for individual values should be left empty, as the information is already available at the vector level and this feels cluttered, i.e. something like: 

<img width="574" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/e6cd897b-a416-4fd2-b087-dd0c414d6dfb">


## @romainfrancois at 2023-06-13T14:44:28Z

Also updating the display value for lists: 

```r
x <- list(1, 2, c("a", "b"))
```

<img width="571" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/f7f12e60-fc1c-4aab-9a50-d52a2e2b8a7c">

instead of relying on `deparse()` : 

```r
> deparse(x)
[1] "list(1, 2, c(\"a\", \"b\"))"
```

## @romainfrancois at 2023-06-13T14:59:10Z

Because we now display individual values, it did not make much sense to keep distinguish between size 1 and size >1 vectors, so size 1 also include the shape: 

```r
x <- 42
```

and can be expanded: 

<img width="579" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/21593995-7e64-4a4b-a2e7-643b8071e9cd">


## @romainfrancois at 2023-06-13T15:04:21Z

This probably is a follow up because this PR is getting bigger than I wanted it to be, but showing individual values does not play well with matrices: 

```r
mat <- matrix(1:12, ncol = 3)
```

<img width="573" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/f796bf63-cf58-40d9-8bfe-9bad9efc1981">
