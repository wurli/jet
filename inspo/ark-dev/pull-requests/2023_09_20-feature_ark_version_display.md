# Add a way to see the Ark version from Positron

> <https://github.com/posit-dev/ark/pull/97>
> 
> * Author: @jmcphers
> * State: MERGED
> * Labels: 

This change adds a new `.ps.ark.version()` method that can be used to query the Ark version, build date, and commit hash.

```
> .ps.ark.version()
               version                 commit                   date 
               "0.1.5"              "49ddb4f" "2023-09-20T15:26:11Z" 
```

Most of the code, though, is written to support a new method that converts a Rust hashmap into an R named vector, which we use to build up the object we return above. 

