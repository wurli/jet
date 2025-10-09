# Use `to_string_lossy()` in `r_str_to_owned_utf8_unchecked()`

> <https://github.com/posit-dev/ark/pull/310>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/2698

Both `r_str_to_owned_utf8()` and `r_str_to_owned_utf8_unchecked()` now use `to_string_lossy()` after calling `Rf_translateCharUTF8()`, since it seems that we can't always count on that to result in valid UTF-8 all the time, especially if the input is:
- Not valid UTF-8 + `"unknown"` encoding + the native OS encoding is UTF-8
- Not valid UTF-8 + wrongly marked as `"UTF-8"` encoding with `Encoding<-`

The invalid UTF-8 now makes it through to the data explorer and shows as the "replacement character", which is correct IMO.

https://github.com/posit-dev/amalthea/assets/19150088/7976df52-238d-4983-aa33-dfd651faa0ca



