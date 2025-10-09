# Add a command-line argument to specify default repositories

> <https://github.com/posit-dev/ark/pull/645>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

This change makes the kernel's behavior around setting default repositories configurable. Before the change, we always set the CRAN repository to `cran.rstudio.com` if it was set to `@CRAN@`. After it, there's a new `--default-repos` command line option which provides additional options for setting repositories:

- **none**, for telling Ark to just leave the option alone
- **rstudio**, for the previous behavior (auto set to `cran.rstudio.com`)
- **posit-ppm**, for setting to Posit's Public Package Manager ("P3M"), which provides convenient pre-built binaries
- a path to a conf file, which can be used to configure multiple repositories at once

If the option is not set, we set the `cran.rstudio.com` repository as we did before, unless there is a `repos.conf` file available in either RStudio or Ark configuration directories, in which case it is read and applied. This behavior lets you share global CRAN repository settings between Positron and RStudio, if desired.

Part of https://github.com/posit-dev/positron/issues/5509 (but doesn't fully address on its own since currently Positron does not pass this command line option; that's coming in a follow-up PR on the Positron side)


