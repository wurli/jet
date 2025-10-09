# Add Windows Application Manifest file

> <https://github.com/posit-dev/ark/pull/178>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

@kevinushey mainly asking you if you can think of any other issues here to be aware of, it seems to work well with some local testing?

This seems to have 2 main effects:

- It allows `ark.exe` to run under something other than the very old `Windows Server 2012` on the VM, i.e. it allows it to run under its actual OS version, `Windows Server 2022`. I think a very similar thing happens on Desktop Windows.
- It tells Windows to use UTF-8 as the `codepage` for `ark.exe`, meaning `l10n_info()` reports `UTF-8` as `TRUE` and `codepage` as `65001`. This allows cli to emit Unicode symbols.

Now, this only work on R 4.2.0 and above, because that's when R started supporting UTF-8 on Windows like this.

RStudio actually has 2 manifest files, which means the have _2 .exe binaries_, i.e. `rsession.exe` and `rsession-utf8.exe`
https://github.com/rstudio/rstudio/pull/10524

If we want to support < R 4.2.0, we may have to do something similar. Or we could consider making 4.2.0 the minimum version of R that we support. We are going to have to have that talk anyways, and decide on some minimum version of R that we want to support, keeping in mind our realistic timeline of when Positron would get out of Beta.

RStudio has a few other things in their `.rc` file, but idk if we want to do all that yet
https://github.com/rstudio/rstudio/blob/d9a61920a87d72846f82f7e45fa5a6f1ff75d33c/src/cpp/session/rsession.rc.in

We also don't have any CMake like infra set up for defining the ark version or license year in one unified place.

## @DavisVaughan at 2023-12-14T13:29:14Z

Merging for now because I am fairly confident it works as expected, but @kevinushey if you see this after break and have additional comments feel free to let me know