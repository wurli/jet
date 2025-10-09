# Implement `did_close()` and per-file diagnostics

> <https://github.com/posit-dev/ark/pull/81>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

A step towards https://github.com/posit-dev/positron/issues/1005

Filled in `did_close()`, which should be useful if we decide we want to regenerate diagnostics for all open files.

Also switched away from using a global variable for `DIAGNOSTICS_VERSION` altogether. I believe we can safely use the per-file `version` for this, which is thread safe due to it being inside a `DashMap` (aka mimicking a `RwLock<HashMap>`). This change allows multiple files to enqueue diagnostics at the same time (i.e. we no longer cancel file A's diagnostics if file B enqueue's theirs), which will also be necessary for regenerating diagnostics for all open files.

I originally thought I was having some kind of deadlock issue with this switch, but I _think_ I may have just compiled a faulty version of ark on my end because the problem went away after a full recompile. It does make me slightly nervous though. I will keep an eye out for anything suspicious over the next few days.

---

https://github.com/posit-dev/amalthea/pull/81/commits/454bae063c22e8b53f29e8828d82a94d169bb1bb contains the switch to using a per-file version check, https://github.com/posit-dev/amalthea/pull/81/commits/a941b77c169cd44016f968ecff3fbede8d1967ed is some further tweaking of `enqueue_diagnostics_impl()` so that it no longer needs to be `async` and has this clear responsibility of `Document in -> Diagnostics out`

