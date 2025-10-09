# Data explorer: Add support for histograms and frequency tables

> <https://github.com/posit-dev/ark/pull/458>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

This implements the OpenRPC protocol for support computing histograms and frequency tables. This is the backend for computing sparklines in the data explorer. 

Addresses https://github.com/posit-dev/positron/issues/4061

TODO:

- [x] Add support for frequency tables (consider factors, and character vectors)
- [x] Add tests for quantile computations.
- [x] Add implementation/tests for the Sturges algorithm
- [x] Plumb with the infra to produce results on the filtered data (if any filters are applied)

## @dfalbel at 2024-08-27T15:31:05Z

I have addressed most comments. Some pending discussions are: 

- Usage of `as i32` to cast from `i64` into an `RObject`. Since the specific usage in this PR is not problematic I didn't change it, but we can revisit in a future PR.
- Usage of `r_parse_eval0` in tests. Opened https://github.com/posit-dev/positron/issues/4497 to track and will do it once #480 is merged.
 