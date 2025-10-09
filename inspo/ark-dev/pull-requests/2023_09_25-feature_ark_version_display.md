# Format Ark build date in local timezone

> <https://github.com/posit-dev/ark/pull/98>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

Currently, the `.ps.ark.version()` command returns the build date in UTC, which is pleasantly unambiguous but not very friendly. This change formats the build date using the local system's timezone to make it more user-friendly.

```
> .ps.ark.version()
                   commit                      date                   version 
                "1a0b7ba" "2023-09-25 09:01:47 PDT"                   "0.1.5" 
```



