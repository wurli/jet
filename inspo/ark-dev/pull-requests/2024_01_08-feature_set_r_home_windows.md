# Highlight potential need to set R_HOME when building ARK (libr-sys, really) on Windows

> <https://github.com/posit-dev/ark/pull/197>
> 
> * Author: @jennybc
> * State: MERGED
> * Labels: 

This is a small quality of life improvement that:

* Makes Cmd/Ctrl + Shift + B work on Windows, i.e. the default build task
* Sets the `R_HOME` env var prior to build

## @DavisVaughan at 2024-01-11T16:05:23Z

Oh, would you also mind structuring the commands as:

```
            "osx": {
                "program": "ark"
            },
            "windows": {
                "program": "ark.exe"
            }
```

i.e. specifically call out `osx` like we do for the launch.json file.

I know that means it won't work out of the box on linux, but that is fine by me because it forces us to actually think about this when we get there