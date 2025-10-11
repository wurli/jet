# Show matrix columns in envpane

> <https://github.com/posit-dev/ark/pull/36>
>
> * Author: @romainfrancois
> * State: MERGED
> * Labels:

Because we now display individual values of vectors (and hence matrices), we need some way to inspect matrices: showing all the values as if they are a vector is not very useful. This does something similar to data frames: showing the columns:

```r
mat <- matrix(1:4, 2)
```

<img width="622" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/cb1c5b36-62ba-4a77-8ca7-4cc1efb36753">



## @romainfrancois at 2023-06-14T15:51:19Z

TODO: update the `display_value` for the matrix itself, Something like `[[1, 2], [3, 3]]` here perhaps.

## @romainfrancois at 2023-06-16T07:58:19Z

```r
mat <- matrix(1:6, 2)
```

<img width="264" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/6d6ff3d4-161f-4770-b66e-3d01c0feab18">


## @romainfrancois at 2023-06-28T15:04:56Z

This can always be improved, but I believe this os good to go. I'll merge once I get the ci ✅.
