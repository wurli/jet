# `break` loop when truncating

> <https://github.com/posit-dev/ark/pull/702>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/6220
Without the `break` we would format the entire list, even though we only display a maximum of 1000 characters.

