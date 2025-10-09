# Implement Jupyter compatible exceptions

> <https://github.com/posit-dev/ark/pull/21>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/rstudio/positron/issues/201

This PR adds some basic support for Jupyter compatible R errors in ark. We support base R errors and rlang errors with separate code paths for each.

The basic idea is to catch an error with `globalCallingHandlers(error =)`, where we install a custom error handler as the last error handler on the stack. We install this error handler once at session startup. The handler is provided the condition object and extracts the error message and traceback, which it sends along to `ps_record_error()` to store for later usage.

On R < 4.0.0, where `globalCallingHandlers()` didn't exist, we simply don't register a handler at all. Since the handler is in charge of telling the Rust code when an R error occurred, this means we will never send an `ExecuteReplyException` and the error will be printed to the console by R instead (as it currently is today).

To prevent R from printing the error (because we do it), we set `options(show.error.messages = FALSE)` at the same time that we register the global calling handler. The dev version of rlang now respects this as well, but the CRAN version does not (more on that below).

If rlang is installed, by default we automatically `entrace()` base R errors into rlang errors with nicer messages and tracebacks. This is currently controlled by a global option called `positron.error_entrace`.

If rlang < 1.1.1.9000 is installed (i.e. if you don't currently have dev rlang), then rlang doesn't support `options(show.error.messages = FALSE)` and there is no way to tell rlang not to print its message to the console. We've worked around this by calling `stop("dummy")` at the end of our global handler, which prevents rlang from continuing on and printing the error message. This should be relatively safe because we install our global handler as the last one on the stack, and it can't have recursive issues because the global handlers are always popped off the stack as soon as they are run, so we can't reenter our error handler. This does mean that `traceback()` and `options(error = recover)` will show the global calling handler frames when rlang isn't new enough, but I don't imagine many people will use `traceback()` since we capture it for them. The other option is to just bail entirely from the global calling handler and not record _anything_. This means rlang would just print the error to the console like it currently does. `traceback()` and `options(error = recover)` would then show the right thing, but we'd get a pretty inconsistent error display depending on the rlang version that is installed. We decided:
- It is more important to have a consistent error display, because everyone sees that
- It is ok for `traceback()` and `options(error = recover)` to show the global handler frames on older rlang versions, because this won't be used by many people and we can just tell them to update rlang if there are questions

---

Some examples 

<img width="365" alt="Screen Shot 2023-06-07 at 1 24 31 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/8bab5849-8cbf-43dd-a9e5-f34800ad3692">

Longer base R traceback, shown with and without entracing

<img width="599" alt="Screen Shot 2023-06-07 at 1 26 29 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/55cf2758-0ba3-48c0-a07a-150ff5dd4cdf">

If base R tracebacks have source references we add them in (rlang does it automatically on that side). We could add a grey color and dim them like rlang does too if we wanted to. Eventually we can make these linkable. (this is with auto entracing off)

<img width="719" alt="Screen Shot 2023-06-07 at 1 28 05 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/5f89cb4d-2360-4218-b401-fcc24921deab">


---

Unresolved issues / notes:

- We have decided that we want to allow experienced users to turn off our global handler if they "know what they are doing." This requires two steps, unregistering it from `globalCallingHandlers()` and setting `options(show.error.messages = TRUE)`. We should probably consider something that makes this easier, possibly a `.ps.*()` function.

- We probably also want some kind of checkbox for `Automatically entrace base R errors with rlang if available` which would be hooked up to `positron.error_entrace`.

- Still need some hyperlink support, of course

- Due to us setting `show.error.messages = FALSE`, collected warnings are shown a bit differently than in RStudio https://github.com/wch/r-source/blob/768e32d10b795d81b52ba511c6ff2030ec0fef66/src/main/errors.c#L883

<img width="412" alt="Screen Shot 2023-06-07 at 1 32 49 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/40743121-b94a-4235-83bb-5736d61db74f">
<img width="605" alt="Screen Shot 2023-06-07 at 1 33 04 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/c58ce293-7fd8-4d41-a4c1-20fc59bf74af">

- There is this super weird and rare thing you can do with `stop()` where you can pass it _any_ kind of condition object and it will signal that condition object as is, then display an error message (but not _signal_ an error). If you pass it a warning condition, it _signals_ a warning so our global calling handler doesn't run, but then it tries to _display_ an error but since `show.error.messages = FALSE` it doesn't display anything. So this ends up not showing anything at all: `stop(warningCondition("oh no"))`. This is super weird and rare, so I'm not too worried about it, but it is worth keeping in mind.
<img width="297" alt="Screen Shot 2023-06-08 at 12 42 23 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/5e272aa2-62c2-4d35-93c3-293b0fd5d169">



## @DavisVaughan at 2023-06-12T12:51:20Z

> Do we have plans to disable the red colour for stderr messages?

This is what I opened https://github.com/rstudio/positron/issues/664 for. It would mean that rlang errors (which typically have ANSI in them somewhere) wouldn't get marked up with red, but base R errors still would.

I think in this case the red color is less about what goes through stdout/stderr and more about whether or not it came from an `ExecuteReplyException` or not

---

I do personally wish that Positron didn't attempt to color _anything_ in red

## @lionel- at 2023-06-12T15:32:02Z

>  It would mean that rlang errors (which typically have ANSI in them somewhere) wouldn't get marked up with red, but base R errors still would.

I think it would be better if all errors looked consistent though? I guess special-casing marked up messages would be a good compromise if we can't remove the red everywhere.