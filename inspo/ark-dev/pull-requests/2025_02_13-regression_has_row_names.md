# `row_labels.is_null()` actually refers to a method checking for a null pointer

> <https://github.com/posit-dev/ark/pull/710>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

This fixes a regression caused by https://github.com/posit-dev/ark/pull/706. `is_null()` is actually checking for a NULL opinter. Now properly check using `r_is_null()`.

## @dfalbel at 2025-02-19T14:04:43Z

I have no idea how we dispatch to `*T`. It was quite unexpected!
I have implemented `is_null` for `RObject` in https://github.com/posit-dev/ark/pull/710/commits/e2f534dab77670668415d1fa6920c5c76ded3766