# Data Explorer: Avoid warnings when summarizing columns

> <https://github.com/posit-dev/ark/pull/471>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

Addresses: https://github.com/posit-dev/positron/issues/4353

It fixes two separate issues:

- We actually want to return `None` when there are no statistics to be computed for numeric values. (Related to https://github.com/posit-dev/positron/issues/4352). This also avoids the warnings that appear in https://github.com/posit-dev/positron/issues/4353

- For datetimes containing invalid timezones such as `x <- as.POSIXct(c("2010-01-01 00:00:00"), tz = "+01:00")`, computing `min` or `max` will raise a warning, so we suppress those.



