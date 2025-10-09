# Data Explorer: Update data explorer comm and adapt to RPC protocol changes related to filtering

> <https://github.com/posit-dev/ark/pull/424>
> 
> * Author: @wesm
> * State: MERGED
> * Labels: 

https://github.com/posit-dev/positron/pull/3904 proposes some refactoring to the `set_row_filters` RPC that makes the filter-specific parameters a union and renames things a bit to enable some code reuse with to-be-implemented column filters. This feels a bit cleaner but we can make some further changes if needed.

I still need to test this out thoroughly locally to make sure it didn't break anything (though the unit tests pass everywhere). 

## @wesm at 2024-07-08T23:14:11Z

I tested this out locally and verified that things still work well after the refactor, so merging. We need to wait to merge https://github.com/posit-dev/positron/pull/3904 until the next Ark release is made -- @lionel- or @dfalbel can you merge that PR when the next Ark version bump is happening or ping me to do it? Just want to avoid things being in a broken state for much time