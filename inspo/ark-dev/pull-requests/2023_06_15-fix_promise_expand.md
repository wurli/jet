# Handle inspection of promises with `PRCODE()` == some object

> <https://github.com/posit-dev/ark/pull/37>
> 
> * Author: @romainfrancois
> * State: MERGED
> * Labels: 

Related to https://github.com/rstudio/positron/issues/626

Example: 

```r
rlang::env_bind_lazy(globalenv(), a = !!mtcars)
```

`a` is a promise to an already evaluated data frame because it has been inlined by `!!`, so as far as the env pane is concerned, this is the same as: 

```r
b <- mtcars
```

<img width="623" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/598fa47b-8daf-45a0-93c1-137928dadbc3">


