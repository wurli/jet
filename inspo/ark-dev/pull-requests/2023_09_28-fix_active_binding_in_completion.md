# Various completion related fixes

> <https://github.com/posit-dev/ark/pull/103>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/1382
Addresses https://github.com/posit-dev/positron/issues/1309

This PR does 4 things:
- Introduces `r_version()` for detecting the running R version, which is surprisingly hard to do
- Introduces `r_env_has()` to determine if a symbol is present in an environment, which has different branches based on the current version of R
- Avoids triggering active bindings during completions, and unifies some of the completion code to try and avoid hitting this in the future
- Forces promises during "search path" completions, i.e. when iterating over the attached namespaces returned from `search()`. Only package environments included in `search()` have promises forced (i.e. not the global env or autoloads)

## @DavisVaughan at 2023-09-29T15:31:08Z

> we could look at the promise expression and force only the lazyload bindings, i.e. those that call into lazyLoadDBfetch()

I think I like this as an _additional_ check, but I don't think it is enough on its own. Lazydata data objects (like `dplyr::starwars`) also show up as `lazyLoadDBfetch()` calls but we _don't_ want to try and force those because they can be pretty slow to force the first time around (it makes `nycflights13::` take 1-2 full seconds to show any completions).

I thought we could also check if the `envir` is a package environment or namespace environment and evaluate the promise if either of those are true AND it is a `lazyLoadDBfetch()` call, but that fails for re-exports because they come from a special "exports" environment.

## @DavisVaughan at 2023-09-29T16:48:29Z

Here is an example of "search path" completions having promises forced

<img width="495" alt="Screenshot 2023-09-29 at 12 37 46 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/6a376685-6e81-470e-882c-61e8b64389e3">

Here I'm proving that lazydata object promises still aren't forced

<img width="615" alt="Screenshot 2023-09-29 at 12 38 15 PM" src="https://github.com/posit-dev/amalthea/assets/19150088/07eb81c2-a454-40bf-873f-c2bbcc180cb2">

And lastly here is coro, where there are some custom promises (i.e. _not_ `lazyloadDBfetch()` calls) that aren't exported. These also are not forced anymore

![Screenshot 2023-09-29 at 11 16 53 AM](https://github.com/posit-dev/amalthea/assets/19150088/bd28947e-8022-4c96-83ef-8ca98403ed74)



## @kevinushey at 2023-09-29T18:19:04Z

Do we have any hope of reading the lazy load database ourselves "by hand", and learning about what objects or object types are stored within?

If not, is it worth the time architecting a feature where we introspect packages in a separate R process so that we can learn about what objects / functions are provided without forcing them in the user's session?

## @DavisVaughan at 2023-09-29T18:48:30Z

@kevinushey we can read the lazy database, but it is incomplete. Really `loadNamespace()` is the key here, from what I can tell. It reads in the lazy database and then "fills out" the raw database with information on re-exports, lazy data objects, s3 methods, the imports and exports of the package, and many other things - so I think we really need to call that to gain access to all of the filled out information, and it does too much to try and reimplement it on our side, i think.

``` r
env <- new.env()
r_path <- file.path(system.file("R", package = "dplyr"), "dplyr")
lazyLoad(r_path, envir = env)
#> NULL
head(names(env))
#> [1] "compute_by"         "warn_join_multiple" "stop_join"         
#> [4] "rename_"            "explain"            "fmt_classes"
```

^ this list is missing things like `as_data_frame()`, a re-export from tibble. That isn't contained anywhere in the database.

---

And the only way to cleanly call `loadNamespace()` is by doing it in a separate R session. Maybe it is worth spending a chunk of time to figure out all of the possible data we might want and how we could send it back over to ark, then storing it in an ark side database for completions (and probably other stuff) like you mentioned.

We probably need an API like, given a package name, return:
- The total list of `::` exports from a package (both exports and re-exports) and their types
- The total list of `:::` internals from a package (defined by the package and its imports) and their types
- The list of lazydata from a package (maybe info on types? might be too slow)
- I think there is also non-lazy data that you can have in a package, so consider what could be done there

Then figure out how to send all that back over to ark. I think "search path" completions (i.e. things on `search()`) and namespace completions (i.e. from `pkg::` or `pkg:::`) could share that.

Would likely be simple if we could use `callr::r()` 😅 

Would be interesting if we could maintain a persistent R session for this to avoid the startup and teardown time of R each time.

---

I would still prefer going ahead and merging this "alpha" kind of solution though, and then immediately turning around and investing time in that if we think it is a good path forward

## @kevinushey at 2023-09-29T22:21:58Z

> I would still prefer going ahead and merging this "alpha" kind of solution though, and then immediately turning around and investing time in that if we think it is a good path forward

I agree with that.

> It seems sort of regrettable that the presence of this active binding prevents the completion database from ?knowing about? or at least revealing devtools::revdep_maintainers() or revdepcheck::revdep_maintainters().

Do you think it's worth special-casing `conflicted` here so that we can then look past the active binding it puts on the search path, and look at whatever happens to be next? Or, should we provide something better than just "active binding" if we see something `conflicted` is putting on the search path?

## @jennybc at 2023-09-29T23:30:18Z

> Do you think it's worth special-casing conflicted here so that we can then look past the active binding it puts on the search path, and look at whatever happens to be next? Or, should we provide something better than just "active binding" if we see something conflicted is putting on the search path?

I'm probably not the main audience for this question, but I think @jmcphers and I decided (before we realized we were rediscovering a known problem) that it was not very appealing to special case conflicted in any way.

If name collisions cause something sub-optimal re: completions for interactive users of the conflicted package, I think that might be part of what we're signing up for (users of conflicted).

## @DavisVaughan at 2023-10-02T14:43:32Z

Agree that it probably isn't worth it to special case conflicted in any way, I added another note here https://github.com/posit-dev/positron/issues/1309#issuecomment-1743151370