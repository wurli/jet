# Record git branch, in addition to the commit hash

> <https://github.com/posit-dev/ark/pull/106>
>
> * Author: @jennybc
> * State: MERGED
> * Labels:

I've found `.ps.ark.version()` very useful in the past 24 hours and I think it will be even more awesome to surface the branch name.

## @jennybc at 2023-09-30T15:28:25Z

Here's how this looks:

```
> .ps.ark.version()
                                      branch
                        "feature/ark-branch"
                                      commit
                                   "96ed51f"
                                        date
                   "2023-09-30 08:26:54 PDT"
                                      flavor
                                     "debug"
                                        path
"/Users/jenny/rrr/amalthea/target/debug/ark"
                                     version
                                    "0.1.10"
```

and, in case you are wondering, the case of detached HEAD:


```
> .ps.ark.version()
                                      branch
                                          ""
                                      commit
                                   "d2b67b4"
                                        date
                   "2023-09-30 07:44:19 PDT"
                                      flavor
                                     "debug"
                                        path
"/Users/jenny/rrr/amalthea/target/debug/ark"
                                     version
                                    "0.1.10"
```

## @jmcphers at 2023-10-02T16:16:12Z

Oh neat!
