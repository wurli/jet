# Add support for configuring a custom Posit Package Manager URL

> <https://github.com/posit-dev/ark/pull/906>
> 
> * Author: @atheriel
> * State: MERGED
> * Labels: 

This commit builds on the existing support for installing packages from Posit Public Package Manager via `--default-repos=posit-ppm` by allowing users to pass a specific Package Manager repository, overriding the hardcoded https://packagemanager.posit.co/cran/latest URL.

This allows users of self-hosted Package Manager installations to benefit from our logic for determining the the appropriate Linux binaries automatically. It also opens the door to users pointing at a specific CRAN snapshot on Public Package Manager rather than `latest`.

Custom URLs are passed via a new `--ppm-repo` option, which is mutually exclusive with `--default-repos` and `--repos-conf`.

Unit tests for the repository URL construction (including for the existing but previously untested logic) are included.

