# Add NOTICE file and generator

> <https://github.com/posit-dev/ark/pull/392>
>
> * Author: @jmcphers
> * State: MERGED
> * Labels:

This change does three things:

1. Adds a NOTICE file to Ark
2. Includes the `LICENSE` and `NOTICE` files in the Ark zipfile
3. Adds an R script that can be used to automatically regenerate the NOTICE file based on the contents of Ark's `Cargo.toml` file.

## @jmcphers at 2024-06-11T17:27:21Z

 > Does checking for LICENSE also catch LICENSE.txt? I noticed that a few use that (same for LICENSE-*)

I would guess it doesn't! I added a check for that (and one for the `main` branch while I was at it) in https://github.com/posit-dev/amalthea/pull/392/commits/9cebee6538ed0f091cecf03531fa567e243ab6a1, but at least for the set of dependencies we use, it didn't make any difference.

## @lionel- at 2024-06-12T12:33:19Z

In release builds the `cp` step was erroring out with:

```
    cp: LICENSE and /Users/ec2-user/actions-runner/_work/amalthea/amalthea/LICENSE are identical (not copied).
    Error: Process completed with exit code 1.
```

This is because the file gets copied multiple times in the matrix build. I tried using `cp -f` but it still fails if target and source are the same. `mv` did not work because the source no longer exists after the first iteration. I briefly attempted to move the license copying to a separate step, but in the end here is what worked: https://github.com/posit-dev/amalthea/commit/8cd1bfbdfffa5f046c53ac66032f0856c97d11b5

ark 0.1.107 is now released!

## @jmcphers at 2024-06-12T16:49:36Z

Thanks for taking care of that `LICENSE` issue!
