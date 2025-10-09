# Stub for rstudio version needs to be a function

> <https://github.com/posit-dev/ark/pull/212>
> 
> * Author: @jennybc
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/2106

`.rs.api.versionInfo()` needs to be a function because it will (or at least can) be `do.call()`'ed by rstudioapi:

https://github.com/rstudio/rstudioapi/blob/7bcdf15d81997c4e901d0eb2f7a67f20ebb6bb93/R/code.R#L68-L112

