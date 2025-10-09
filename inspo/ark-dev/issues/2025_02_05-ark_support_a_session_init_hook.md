# ark: Support a session init hook

> <https://github.com/posit-dev/ark/issues/697>
> 
> * Author: @jennybc
> * State: OPEN
> * Labels: 

Follow-up work related to posit-dev/positron#2070 and renv support.

RStudio supports a session init hook that runs after R is fully initialized:

https://github.com/rstudio/rstudio/blob/8ebe5431ca2fb3c9719b70d4fbc45201d226c79c/src/cpp/session/modules/SessionRHooks.hpp#L19

https://github.com/rstudio/rstudio/blob/8ebe5431ca2fb3c9719b70d4fbc45201d226c79c/src/cpp/session/SessionMain.cpp#L857

https://github.com/rstudio/rstudio/blob/8ebe5431ca2fb3c9719b70d4fbc45201d226c79c/src/cpp/session/modules/SessionRHooks.hpp#L36-L39

https://github.com/rstudio/rstudio/blob/8ebe5431ca2fb3c9719b70d4fbc45201d226c79c/src/cpp/session/modules/SessionRHooks.R#L16-L31

---

Good resource on R startup: https://rstats.wtf/r-startup

---

People typically use this sort of hook to schedule startup-ish code that makes use of, for example, `readline()`, `menu()` or rstudioapi. Our immediate use case here in Positron is related to renv, where it would be nice to schedule the "hey you need to do `renv::restore()`; shall we do it? interaction for execution after R has been fully stood up.

---

GitHub search for uses of `rstudio.sessionInit`:

<https://github.com/search?q=rstudio.sessionInit&type=code>

## @jennybc at 2024-06-07T19:48:17Z

Notes for when we work on this: I think it would be interesting to make it possible for a frontend to advertise the name of its session init hook. In the current world, the most obvious place for such functionality to live is rstudioapi and then RStudio and Positron could both implement the associated function. That will reduce the need for packages to have code specific to RStudio vs. Positron.

This note is motivated by laying the groundwork in renv for playing more nicely in Positron. In the near term, we're going to be happy to prevent R's failure to launch (which is https://github.com/posit-dev/amalthea/pull/383) and to recommend a version of renv that emits a message telling the user to run `renv::restore()` (which is https://github.com/rstudio/renv/pull/1915). But we'd really like to have the same behaviour as RStudio, which is to defer interactive startup activities to a session init hook.

Update: Hmmm, upon further thought, one of the main reasons for RStudio's session init hook is because you can't call rstudioapi during startup. But I suspect that only applies to certain functions, i.e. to the ones that genuinely interact with RStudio. A simple function that just returns a string (the name of the session init hook) might be fine.