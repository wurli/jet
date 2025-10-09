# Fix up formatting for S3 objects

> <https://github.com/posit-dev/ark/pull/916>
> 
> * Author: @juliasilge
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/8053

With this PR, you can see `Surv` objects in the Data Explorer:

<img width="1026" height="900" alt="Screenshot 2025-09-03 at 9 29 43 PM" src="https://github.com/user-attachments/assets/69e527b5-8297-4b72-94ea-2883e360c272" />

And we no longer get this error I point out here, when you click to expand a `Surv` object in the Variables Pane:

<img width="514" height="385" alt="Screenshot 2025-09-03 at 8 38 45 PM" src="https://github.com/user-attachments/assets/5a303798-06a5-4e13-913a-bdf3fc61d93b" />


Good news on both fronts! 🎉 

This manhandling of `dim()` was introduced for https://github.com/posit-dev/positron/issues/1862, also motivated by `Surv` objects. However, I think we must have updated other pieces that handle this, because those original examples are fine even after I remove this code:

<img width="677" height="382" alt="Screenshot 2025-09-03 at 9 32 18 PM" src="https://github.com/user-attachments/assets/99baafb6-04bc-4023-9970-11353fa96ac3" />

No panics! More good news 🎉 

There is still a little bit of bad news. Now when you click to expand a vector of `Surv` objects in the Variables pane, there is a _new_ error:

<img width="487" height="466" alt="Screenshot 2025-09-03 at 9 35 03 PM" src="https://github.com/user-attachments/assets/0cb10aa2-3cae-4a72-8554-68c4e912ae77" />

Try with this code:

```r
library(tidyverse)
library(survival)

res <- modeldata::cat_adoption |>
    slice_sample(n = 5) |>
    mutate(
        event_time = Surv(time, event),
        .keep = "unused",
        .before = everything()
    )
```

And the error message has:

> Error expanding variable item: Failed to process positron.variables request: Error evaluating 'harp_subset_vec(structure(c(76, 95, 28, 104, 18, 1, 1, 1, 1, 1), dim = c(5L, 2L), dimnames = list(NULL, c("time", "status" )), type = "right", class = "Surv"), 6:10)': subscript out of bounds R backtrace: 0. harp_subset_vec(structure(c(76, 95, 28, 104, 18, 1, 1, 1, 1, 1), dim = c(5L, 2L), dimnames = list(NULL, c("time", "status" )), type = "right", class = "Surv"), 6:10)

Notice that it is trying to subset `6:10` here, which doesn't exist. I suspect this is related to https://github.com/posit-dev/ark/pull/646, and how we changed to subsetting _before_ formatting.

@dfalbel could you take a look at this and see if you have suggested changes we could make here to make this work in the Variables pane? It would be nice to just get it all fixed and write some tests to make sure it does not regress.

## @juliasilge at 2025-09-04T19:13:29Z

Thanks, @dfalbel!

- In 539b8ae49bef26ba36199a954297ef459e546859 I am showing what I tried using the `.ark.register_method()` approach, but adding these keeps R from starting up ("R 4.5.0 failed to start up (exit code -1) The process exited abnormally (signal: 6 (SIGABRT))")
- In 2a132ddd2515335436bdd9e74a56a49f99963279 I am showing how I started to sketch out dispatching on `length()` but I could not get that to work because a `Surv` object is a _matrix_; I could not get the Variables pane handling for matrices untangled from how/when we format vs. subset.

I tend to think it might be better to use `.ark.register_method()` and make special handling for `Surv` objects? But open to what you think.

## @juliasilge at 2025-09-05T19:07:26Z

Amazing, @dfalbel, thank you so much!

### Release Notes

#### New Features

- N/A

#### Bug Fixes

- R: Fixed how `Surv` objects are represented and formatted in the Data Explorer and Variables pane


### QA Notes

For code like this:

```r
library(tidyverse)
library(survival)

res <- modeldata::cat_adoption |>
    slice_sample(n = 15) |>
    mutate(
        event_time = Surv(time, event),
        .keep = "unused",
        .before = everything()
    )
```

You can:

- run `View(res)` and see the `Surv` column in the Data Explorer
- click to expand the `res` object in the Variables pane, including expanding the `event_time` column (which is like a matrix nested inside the dataframe)

<img width="1512" height="996" alt="Screenshot 2025-09-05 at 1 07 15 PM" src="https://github.com/user-attachments/assets/200f2d23-0b71-483b-b9c7-cb0f008f9b70" />
