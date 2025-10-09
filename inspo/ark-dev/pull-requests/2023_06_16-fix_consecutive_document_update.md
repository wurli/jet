# Only apply consecutive changes when updating document states

> <https://github.com/posit-dev/ark/pull/45>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/rstudio/positron/issues/681

This really shot up to Private Alpha priority for me because it was happening all the time, especially when typing `options()` in the console for some reason.

This is really a follow up to this commit https://github.com/rstudio/positron/commit/6c0f1a97f3630c394caa8aed2207a37cd3246960, which is related to https://github.com/rstudio/positron/issues/340.

In that commit, we handle out of order changes by storing them until we see the next consecutive change, but we could run into a scenario like this:

```
got version 1
apply version 1

got version 4
defer version 4 

version 2
apply version 2
apply version 4 (here is the issue!)
```

In other words, once we hit the next consecutive change we just unload all the changes without checking that they are also consecutive.

This PR fixes this by only applying as many consecutive changes as possible, and then we re-push the remaining changes that we can't apply right now as `pending` again.

Here is me catching the problem with some extra logging

<img width="1252" alt="Screen Shot 2023-06-16 at 10 06 29 AM" src="https://github.com/posit-dev/amalthea/assets/19150088/d17e6be3-c1e8-45f1-a6dc-25e4f0db6541">

And here it is now with the fix

<img width="1366" alt="Screen Shot 2023-06-16 at 11 02 11 AM" src="https://github.com/posit-dev/amalthea/assets/19150088/72638c31-53f4-48b3-94da-b42bf135a92c">




## @DavisVaughan at 2023-06-22T16:02:49Z

@lionel- and I discussed offline and it seems like `take()` is probably the cleanest way to do this still