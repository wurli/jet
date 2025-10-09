# Add back correct S4 support to the variables pane

> <https://github.com/posit-dev/ark/pull/658>
> 
> * Author: @dfalbel
> * State: MERGED
> * Labels: 

The change in [this PR](https://github.com/posit-dev/ark/pull/630#discussion_r1853928076) broke S4 support. This PR restores it, but replaces the previous FormattedVector approach with a custom method, as the former seemed incorrect.

Adresses https://github.com/posit-dev/positron/issues/5685

