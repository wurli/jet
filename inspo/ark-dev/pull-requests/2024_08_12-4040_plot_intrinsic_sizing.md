# Intrinsic plot sizing (placeholder implementation)

> <https://github.com/posit-dev/ark/pull/470>
> 
> * Author: @seeM
> * State: MERGED
> * Labels: 

This PR provides a minimum required change to support a corresponding Positron PR: https://github.com/posit-dev/positron/pull/4323 i.e.

1. It always responds with an empty result to `get_intrinsic_result` on the plot comm, which disables the feature for the plot in Positron.
2. It `bail`s on `render` with an unspecified `size`. Those shouldn't happen given the Positron-side changes mentioned in point 1.

