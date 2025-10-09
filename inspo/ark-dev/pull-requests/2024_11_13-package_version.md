# Expose package version info

> <https://github.com/posit-dev/ark/pull/625>
> 
> * Author: @jennybc
> * State: MERGED
> * Labels: 

Required by https://github.com/posit-dev/positron/pull/5365 to address https://github.com/posit-dev/positron/issues/1957

There are times when positron-r needs multiple pieces of related info about a package installation. I know it looks funny to get the package version _and_ to pass back info about whether that version satisfies a minimum version requirement, but it's impractical to do that comparison in positron-r (i.e. R package versions don't play nicely with semver).

And to message about a package that is installed, but at insufficient version, you need all 3 of these pieces of info: installed version, required version, and whether that requirement is met.

