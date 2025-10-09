# Data Explorer: Display row names for matrix

> <https://github.com/posit-dev/ark/pull/706>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

Checking for the `row.names` attribute is not enough to figure out if an object has `row.names`. Especially, matrices use the `dimnames` attributes to store this information. So we now call into `base::row.names()` to check for the existence of `row.names`.

Adresses https://github.com/posit-dev/positron/issues/6287

