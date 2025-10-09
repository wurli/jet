# Fix `r_lock!` usage in `RDataViewer`

> <https://github.com/posit-dev/ark/pull/2>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/rstudio/positron/issues/589

- Used `r_lock!` as needed before calling the R API
- Tweaked `extract_columns()` to take `SEXP` rather than `RObject` to avoid some extra layers of `PROTECT()`. Plus i think it looks a little cleaner?

## @DavisVaughan at 2023-05-19T13:25:35Z

Some "proof" that we do indeed need to `r_lock!` here in the data viewer (`flights` is large so it takes the viewer a minute to work through it):

https://github.com/posit-dev/amalthea/assets/19150088/98b8a635-fd5b-4744-be38-43e7c521c361


I can confirm that with this PR it no longer crashes, and the console "waits" until the `r_lock!` has been released by the viewer