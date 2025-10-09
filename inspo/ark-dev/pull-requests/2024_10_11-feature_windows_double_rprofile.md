# Ensure R doesn't run the `.Rprofile` on Windows

> <https://github.com/posit-dev/ark/pull/581>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/4253

Have I mentioned how much I love being able to write these integration tests?

---

The first part of my analysis in https://github.com/posit-dev/positron/issues/4253#issuecomment-2271978659 is not quite right. We do call `cmdlineoptions()` but we _don't_ actually pass it the command line arguments like `--no-init-file`.

Here's what actually used to happen:
- `RMain::start()` is called
- `setup_r()` is called
    - `R_SetParams()` is called to set our hooks. Because we used `R_DefParamsEx()` to initialize the params object, this defaults `->LoadInitFile` and `->LoadSiteFile` to `TRUE`.
    - `setup_Rmainloop()` runs. This runs both the user and site level `.Rprofile` because of the above bullet.
-  Back in `RMain::start()`, we don't see `--no-init-file` in the command line args, so ark decides it should load the user `.Rprofile`. We do so, meaning we've now run it twice.

The simple solution is to set `LoadInitFile` and `LoadSiteFile` to `FALSE` alongside the other `param` hooks. This ensures that ark is the only one who can run the user's `.Rprofile`s.

---

Note that this doesn't occur on Mac or Linux because we actually _do_ pass on the user's command line arguments through to `Rf_initialize_R()` there, and that respects the `--no-init-file` that we always pass through. On Windows we are not passing through the user's command line arguments _at all_, and we will probably move away from this on Unix too soon (https://github.com/posit-dev/ark/issues/708).

