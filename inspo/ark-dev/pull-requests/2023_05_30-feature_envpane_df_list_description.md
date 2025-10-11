# [env pane] description for data frames and lists

> <https://github.com/posit-dev/ark/pull/15>
>
> * Author: @romainfrancois
> * State: MERGED
> * Labels:

addresses https://github.com/rstudio/positron/issues/638

Data frames mimics the display used in python, and list leverage `deparse()`, although I'm not sure about this):

```r
> x <- list(foo = "bar", baz = "quux")
> mtcars <- mtcars
> iris <- dplyr::group_by(iris, Species)
```

gives:

<img width="671" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/eaa0eae8-9394-429c-bdfd-b831c96e5140">



## @romainfrancois at 2023-05-30T13:45:37Z

Using `deparse()` might be too simple here, perhaps we should rather have something recursive, here is what `y <- list(a = mtcars)` looks like:

<img width="337" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/a019fb97-42a1-4cb9-9018-f81339758d07">

(although it's better than nothing), but we'd probably want to be something like `list(a = <data.frame>)`...

<img width="337" alt="image" src="https://github.com/posit-dev/amalthea/assets/2625526/dc69cb95-be53-40dd-8319-4ba4deb17e97">


## @romainfrancois at 2023-05-30T15:31:26Z

Merging right now so that it's merged before #11 but will follow up on https://github.com/rstudio/positron/issues/638
