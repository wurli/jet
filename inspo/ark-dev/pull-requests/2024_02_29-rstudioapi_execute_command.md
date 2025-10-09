# Implement `executeCommand` for rstudioapi as OpenRPC contract

> <https://github.com/posit-dev/ark/pull/256>
> 
> * Author: @juliasilge
> * State: MERGED
> * Labels: 

To go along with posit-dev/positron#2356

You can try it out with:


```r
rstudioapi::executeCommand("potato")
rstudioapi::executeCommand("potato", quiet = TRUE)
rstudioapi::executeCommand("activateConsole")
```

## @juliasilge at 2024-03-01T16:31:05Z

TODO:

- [x] Bump ark version in this PR