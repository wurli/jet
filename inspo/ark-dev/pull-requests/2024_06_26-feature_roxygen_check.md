# Document soft-dependencies of Ark on R packages and warn if not met

> <https://github.com/posit-dev/ark/pull/417>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Adds documentation for our package min-version dependencies: https://github.com/posit-dev/ark/blob/feature/roxygen-check/doc/package-dependencies.md

These requirements are usually for enhancements or for bugs that we have worked around. There is one bugfix in roxygen that is annoying, so we now warn on load:

<img width="540" alt="Screenshot 2024-06-26 at 10 06 59" src="https://github.com/posit-dev/ark/assets/4465050/bdbcbdce-32ff-4b02-891c-ae413b2fdb5a">

To support the nice cli output I've added some tools ported from rlang's `standalone-cli.R` file.

The fix is not on CRAN yet but @hadley will do a release soon.

@jennybc The enhancement/bugfixes for usethis are missing from this list. Do you remember what they are about?

## @jennybc at 2024-06-26T17:31:33Z

I think I put usethis on that list purely to exercise dev usethis + Positron more, as dev usethis uses cli heavily in its UI. I am working towards a near-term usethis release. But I think that's just going to give people a nicer, prettier life. It's not actually a matter of function.