# Don't tie the diagnostics version to the document version

> <https://github.com/posit-dev/ark/pull/75>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/rstudio/positron/issues/999
Addresses https://github.com/rstudio/positron/issues/997

I think our global `VERSION` was too closely tied to the document version.

If the console was at "document" version 200, but `foo.R` was at document version 50, then we will _never_ run diagnostics for `foo.R` since we essentially always end up setting the global `VERSION` to the maximum document version we see, and we always bail from running the diagnostics on any document version that is lower than `VERSION`

Instead I've given the global `DIAGNOSTICS_VERSION` its own separate counter. It increments anytime anyone requests diagnostics and now serves the sole purpose of aborting diagnostic generation if we see that the global version has been incremented elsewhere by the user continually typing.

The previous `VERSION` implementation had some checks to make sure that we don't try and generate diagnostics for a change that is "old", and I think I've kept the spirit of that by moving a similar check back up into `did_change()` that only tries to run diagnostics if `on_did_change()` was able to successfully bring the document all the way up to the "change" version.

Some proof that document diagnostics are regenerated now:

https://github.com/posit-dev/amalthea/assets/19150088/19c9cdb3-9bb1-4110-a799-66b706f1e3e4




## @DavisVaughan at 2023-08-09T20:29:09Z

The one downside of this approach is that if you type in the console _right after_ you finish typing in `foo.R` then it can cancel the pending diagnostics generation that was about to happen for `foo.R`

We could try and use a global hash map with the key being the URL and the value being the counter to try and avoid this, but it was somewhat complicated to figure this out and the current behavior doesn't seem awful