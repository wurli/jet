# Limit the number of bins for histograms to avoid crashes

> <https://github.com/posit-dev/ark/pull/655>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

Addresses: https://github.com/posit-dev/positron/issues/5744 by limiting the maximum amount of bins that are returned to the front-end. This info was already provided by the front-end but we didn't respect it - instead using the result of `nClass` methods.

