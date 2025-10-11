# showing only first class

> <https://github.com/posit-dev/ark/pull/38>
>
> * Author: @romainfrancois
> * State: MERGED
> * Labels:

Followup to https://github.com/rstudio/positron/issues/638#issuecomment-1572521749

```r
x <- dplyr::group_by(mtcars, cyl)
```

![image](https://github.com/posit-dev/amalthea/assets/2625526/44e0cc07-e863-4c01-85f8-174b34615154)

Is that redundant to see `grouped_df`  twice though ?

## @DavisVaughan at 2023-06-15T12:25:13Z

> Is that redundant to see grouped_df twice though ?

When we first added this "data frame label" feature, I did think showing both`[32 rows x 11 columns] <grouped_df>` and `grouped_df [32, 11]` was pretty redundant

An alternative for the label that may be more useful is to show the first few column names?

## @romainfrancois at 2023-06-15T14:30:58Z

> An alternative for the label that may be more useful is to show the first few column names?

Yeah I think this is more useful. I'll merge this one now, and follow up
