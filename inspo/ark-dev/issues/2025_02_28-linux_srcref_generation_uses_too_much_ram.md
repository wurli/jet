# linux: srcref generation uses too much RAM

> <https://github.com/posit-dev/ark/issues/726>
> 
> * Author: @aymennasri
> * State: OPEN
> * Labels: 

Using Zed and Ark together, the reported used memory is approximately 861Mb which is a lot for a kernel that runs REPL cells on Zed, not sure if this is normal.


## @lionel- at 2025-02-27T10:34:39Z

Can you try with the latest dev version please, we did some work to improve memory usage recently.

Also please describe the state of your session (how many packages used etc).

## @aymennasri at 2025-02-27T13:59:45Z

Will update and inform you, and it's a clean session with a single file opened and after running a simple REPL cell the consumption of RAM can be seen gradually increasing from 115Mb to 767Mb for the test i just did.

Edit: Code used to test it:

```
#%%
library(mirai)

task <- mirai({
  Sys.sleep(5)
  mean(rnorm(100))
})

print("I’m not blocked!")

task_result <- task[]
```

## @aymennasri at 2025-02-27T14:12:33Z

After testing the latest dev version, it does seem to now stabilize around 110Mb 

## @lionel- at 2025-02-28T08:37:20Z

Even after triggering completions? And loading e.g. `tidyverse` and triggering completions again? This should cause memory to spike.

## @lionel- at 2025-02-28T08:38:58Z

Interesting that srcref generation (which is now lazy and doesn't cause increased memory usage on startup) seems to take up much more memory on Linux 

## @lionel- at 2025-02-28T08:40:11Z

oh but in zed you won't use any of the LSP features, so you won't see increased mem usage from completions.

## @aymennasri at 2025-02-28T15:28:10Z

The completions do add a bit of memory consumption but not that much. However, whenever i call `ggplot()` the consumption spikes by ~20Mb (not consistent and `plot()` *can be?* affected by this)