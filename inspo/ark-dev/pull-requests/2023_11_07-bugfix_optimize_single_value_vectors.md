# Optimize display type of simple vectors with a single dimension and a single value

> <https://github.com/posit-dev/ark/pull/139>
> 
> * Author: @softwarenerd
> * State: MERGED
> * Labels: 

If you execute the following block of code in Positron:

```R
cplx_val <- 1 + 2i
dbl_val <- 100
int_val <- 100L
lgl_val <- TRUE
str_val <- "T"
raw_val <- charToRaw(str_val)
```

Each of the values is displayed as a simple vector of length one and is thus expandable / collapsable:

<img width="1792" alt="Positron-Before" src="https://github.com/posit-dev/amalthea/assets/853239/d70fcc00-56f9-45fc-b8b2-15f1caa534c2">

This is confusing and unnecessary. There's no point in making these values expandable / collapsable.

With this PR, simple vectors of length one will now have a `display_value` of just the `r_vec_type`. Additionally, `has_children` will report false for single-valued vectors of type `LGLSXP | RAWSXP | STRSXP | INTSXP | REALSXP | CPLXSXP`.

Here's what this looks like:

<img width="1792" alt="Positron-After" src="https://github.com/posit-dev/amalthea/assets/853239/b0632113-6ad0-4d2d-95cc-96daefabbd1d">

I believe I've correctly coded all this, but I definitely need more experienced eyes to check my work.

Thanks!

## @DavisVaughan at 2023-11-07T14:47:33Z

I actually think the current behavior is more consistent with how vectors work in R. We don't have scalar types, all of these types are always vectors. These just happen to be "vectors of length 1" and I personally don't think we need to try and treat these specially

## @jjallaire at 2023-11-08T02:21:42Z

I think I agree that we should treat them specially (note that in RStudio we did treat them specially and I don't recall anyone ever complaining about that, including many members of R core and the tidyverse team who used RStudio for years). While I agree that there are pedantically no scalars in R, there are certainly variables that the user semantically intends to be scalars, and I think it's more helpful to show them that way.

## @jjallaire at 2023-11-08T02:49:07Z

I could be getting this wrong, but here's an example of R giving a simpler presentation to a scalar for printing:

```r
> x = 10
> str(x)
 num 10
> x = c(5,10)
> str(x)
 num [1:2] 5 10
```

I guess I would have expected `num [1] 10` or `num [1:1] 10` for the scalar case if R was being fully consistent. There appears to be some special effort to single out the scalar as "just a num" for printing. 

Anyway, realize we are wading into tricky territory here, but I'm less worried about the experienced R user being "confused" by a lack of consistent vector treatment (as they have demonstrably not been confused by RStudio) than the less experienced user who is semantically trying to express a scalar "confused" by the fact that it shows up as an array.

Of course a middle ground could be to just print the value of `str()` (effectively moving type from the right column to before the value). That's what the VS Code R Extension does:

<img width="409" alt="Screen Shot 2023-11-07 at 9 45 33 PM" src="https://github.com/posit-dev/amalthea/assets/104391/1ecc6036-7600-4969-8bb7-1f5c9cb9b32d"> 
<img width="271" alt="Screen Shot 2023-11-07 at 9 45 45 PM" src="https://github.com/posit-dev/amalthea/assets/104391/2a8d662e-752f-4eaf-8612-38539e2db30b">


