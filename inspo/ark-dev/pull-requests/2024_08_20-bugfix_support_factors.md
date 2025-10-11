# Data Explorer: Add support for factors summary stats

> <https://github.com/posit-dev/ark/pull/478>
>
> * Author: @dfalbel
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/2161

Without this PR we fail to compute stats for factors. They are treated as strings by the front-ent, thus they get into the codepath of calling `summary_stats_string()` and `nzchar` requires a character vector.



