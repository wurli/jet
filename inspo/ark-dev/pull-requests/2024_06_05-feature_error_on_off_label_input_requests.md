# Take full control of sourcing `.Rprofile` and `.Rprofile.site`

> <https://github.com/posit-dev/ark/pull/383>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/2070
Addresses https://github.com/posit-dev/positron/issues/3083
Addresses https://github.com/posit-dev/positron/issues/1481

This PR solves the renv problem on the ark side (https://github.com/posit-dev/positron/issues/2070), along with a number of other startup related issues.

After exploring a number of possible solutions, the most robust thing to do seems to be to manually take the reigns on sourcing `.Rprofile` and `.Rprofile.site`, and to now always pass `--no-init-file` and `--no-site-file` to R, so it never will. This:
- Gives ark a chance to fully start up, including a chance to run `libraries.initialize_post_setup_r()` to initialize global constants like `R_BaseEnv` and `R_NilValue`. This is important because if R sourced the `.Rprofile`, then arbitrary user code could wrap around into ark internals that utilize these global constants before they are initialized (indeed that was happening with `readline()`!)
- Lets us capture the R startup banner completely separate from any output emitted from an `.Rprofile` file. This way on restarts and when `quiet = true` is set, we don't also swallow stdout from the user's `.Rprofile`.
- Lets us run the IDE provided `startup.R` file before the `.Rprofile`. This is important because it sets cli specific global options, enabling cli hyperlinks and color to work in the user's `.Rprofile`.
- Makes it easier to override `readline()` and `menu()`, see below.

We now _override_ both `readline()` and `menu()` while sourcing the `.Rprofile` files. Since we are in control over sourcing them, we can use our existing machinery for unlocking and overriding the binding in the package and namespace environments. Both of those functions now return an informative error, _unless_ `readline()` is called when `getOption("renv.autoloader.running")` is active, which is our signal that we are in the renv autoloader. In other cases, any error thrown while sourcing the `.Rprofile` is propagated back to the user's Console through an IOPub Stream message over Stderr. This seems to be our best option, we can't even use the UI Comm at this point (like, to show a toast message), because it is unlikely to be connected to the frontend yet (I tried). This is also more general to other frontends.

Here is a fresh clone of an renv-using git repo, now starting up (using our special `readline()` hook that returns `"n"`):

<img width="598" alt="Screenshot 2024-06-05 at 9 07 16 AM" src="https://github.com/posit-dev/amalthea/assets/19150088/5df29bcb-6889-4cb2-95e2-ae7d34a7fe73">

Here is what happens when you error in your `.Rprofile`:

<img width="565" alt="Screenshot 2024-06-05 at 9 06 41 AM" src="https://github.com/posit-dev/amalthea/assets/19150088/a08cb210-06e6-483c-af4d-5b08ad03f007">

Here is the `readline()` override error in particular:

<img width="713" alt="Screenshot 2024-06-05 at 9 06 14 AM" src="https://github.com/posit-dev/amalthea/assets/19150088/a88afeed-0098-412c-a885-7f1d34c5f9cd">

And here are cli colors and links being emitted from an `.Rprofile` now. Notice how on restart we don't emit the R banner, but we now _do_ emit the `cat()` statements from the `.Rprofile`!

<img width="1201" alt="Screenshot 2024-06-05 at 9 17 45 AM" src="https://github.com/posit-dev/amalthea/assets/19150088/3cb0af47-6048-48b5-9814-41cc8962de5d">


I believe there is a minor step backward in this PR. Previously if you set the `prompt` or `continue` global option from within your `.Rprofile`, then it would get picked up on the first `read_console()` iteration where `complete_initialization()` was run. We now run `complete_initialization()` before sourcing the `.Rprofile` files, so now the prompt config change doesn't get picked up until after you complete your first `execute_request` (i.e. when it sends the updated prompt state back to the frontend before doing `self.active_request = None` to clear that first `execute_request`). With some more tinkering we can probably get the ordering right on this. Possibly in this or a follow up PR.



