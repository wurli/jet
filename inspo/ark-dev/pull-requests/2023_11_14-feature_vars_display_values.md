# Display values of list and environment variables

> <https://github.com/posit-dev/ark/pull/145>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

~Branched from #144~

I currently find the display of lists and environments a bit confusing. I think for these two reasons:

- Lists are represented with square brackets but not consistently. Only inner lists are represented this way, with an additional wrapping to represent the outer list (hard to explain but should be clearer in the screenshots below).

- Environments are also containers but do not have a delimiter representation.

To address this, I propose to always enclose lists in square brackets, and to enclose environments with curly brackets, which is sort of consistent with JS/JSON objects. Here is how it looks with

```r
ll <- list(1, list(2), c = 3)
ee <- list(rlang::env(a = 1, b = 2))
```

**Before:**

<img width="406" alt="Screenshot 2023-11-14 at 15 39 54" src="https://github.com/posit-dev/amalthea/assets/4465050/c4afeff5-7a11-4aba-a2ad-568d090630e8">
<img width="398" alt="Screenshot 2023-11-14 at 15 40 11" src="https://github.com/posit-dev/amalthea/assets/4465050/56a04726-d59d-447a-873d-07506d759687">


**After:**

<img width="404" alt="Screenshot 2023-11-14 at 15 38 00" src="https://github.com/posit-dev/amalthea/assets/4465050/8c557942-c7cd-4230-a56f-aa4a5c06ceb5">
<img width="405" alt="Screenshot 2023-11-14 at 15 39 02" src="https://github.com/posit-dev/amalthea/assets/4465050/4f6019ea-0f82-481e-a053-54a08a0be279">



In addition, this PR also includes the name of the environment if it has one.

```r
a <- list(asNamespace("utils"), globalenv())
```

**Before:**

<img width="405" alt="Screenshot 2023-11-14 at 15 44 18" src="https://github.com/posit-dev/amalthea/assets/4465050/0188739f-5e95-4716-b823-5758adf0bfd0">


**After:**

<img width="407" alt="Screenshot 2023-11-14 at 15 45 28" src="https://github.com/posit-dev/amalthea/assets/4465050/f0677103-e4c0-41e8-a391-467c5af8df50">



