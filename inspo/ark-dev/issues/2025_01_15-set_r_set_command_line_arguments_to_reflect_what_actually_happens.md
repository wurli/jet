# Set `R_set_command_line_arguments()` to reflect what actually happens

> <https://github.com/posit-dev/ark/issues/670>
> 
> * Author: @DavisVaughan
> * State: OPEN
> * Labels: 

Some packages like `startup::startup()` utilize `commandArgs()` to make decisions. For example, that package checks to see if `--no-site-file` is in the list of command args. If it is, then it refuses to try and load the "extended" site profiles that that package specially supports. Same with `--no-init-file`.

The problem is that we "manually" run both `.Rprofile.site` and `.Rprofile` for the user once ark has set up enough to be able to handle running arbitrary scripts. We have to forcibly tell R _not_ to run these by setting `--no-site-file` and `--no-init-file`, even if we eventually run them. This confuses the startup package.

We should probably retain a copy of `args` that reflect the "real" state of the world and call `R_set_command_line_arguments()` after calling `Rf_initialize_R()` with the "fake" `args`. This is what `commandArgs()` ends up pulling from.

A test plan for this would be an integration test that checks `commandArgs()` after startup with a test client that allows both the user and site level R profiles to run. The command args should not contain `--no-site-file` nor `--no-init-file`

