# Invalidate filters when data is updated

> <https://github.com/posit-dev/ark/pull/366>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

Filters can become invalid when the column they refer to is removed or when the data type of the column they refer to is changed and no longer supported.

Addresses https://github.com/posit-dev/positron/issues/3141

