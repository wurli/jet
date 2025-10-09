# Consistently reset vmax stack after translation

> <https://github.com/posit-dev/ark/pull/105>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

This is a follow-up to the discussion in #99.

- New `r_str_to_owned_utf8()` and `r_str_to_owned_utf8_unchecked()` helpers to convert R strings.

- `r_translate_string()` has been renamed to `r_chr_get_owned_utf8()`. This naming is consistent with `r_chr_get()` in the rlang API and the new helpers.


