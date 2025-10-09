# Fix bug when formatting dates containing `NA`s for the data explorer

> <https://github.com/posit-dev/ark/pull/421>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/3734

The issue was that converting from `RObject` to `Vec<String>` does not allow for the character vector to contain NA`s and unexpectedly `format()` might return character vectors containing NA's, instead of formatting them with eg `"NA"`.

This happens for POSIXct objects, for example:

```
> format(as.POSIXct(c(NA)))
NA
```

We still want to go over `is.na(object)` because factors will format `NAs` with `"NA"` and we won't be able to differentiate from a level called `"NA"`, if it ever happens.

