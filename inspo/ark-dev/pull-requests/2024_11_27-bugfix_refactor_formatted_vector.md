# Refactor `FormattedVector` for faster formatting of S3 objects

> <https://github.com/posit-dev/ark/pull/646>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

The variables pane uses the `FormattedVector` API to format vector elements to be displayed. One important assumption that we do when using this API is that it's lazy, in the sense that we create an iterator over the elements of a vector and we only pay the formatting price for the values we actually iterated. 

This is true for atomic vectors, for which we iterate using R's C API eg, extracting values with `REAL_ELT` and formattinng them in Rust. However, this is not true for any vector that has a class (except factors), in this case we need to use the `format` function, and we were actually formatting the entire vector before being able to iterate over the formatted values.
This is very costly if we need for format very large vectors, as when we expand the data.frame in https://github.com/posit-dev/positron/issues/3628#issuecomment-2498863196

This PR refactors the `FormattedVector` API, with the main change being:

- We no longer format the entire vector upon construction of the `FormattedVector`
- Instead, we'll only format the vector when creating the `iter()`
- We provide a few different iterators: `iter()`, `iter_n()`, `column_iter()`, `column_iter_n()`. Those suffixed with `n` will only format the requested part of the vector.

As a consequence, now creating the `iter()` returns a `Result`, so we need to adapt in a few places. 
Now, most of the times in the variables pane we'll use `iter_n()` to only format the first `N` elements, that are necessary
to create the display value in the variables pane.



