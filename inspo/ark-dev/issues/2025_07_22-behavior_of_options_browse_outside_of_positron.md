# Behavior of `options(browse =)` outside of Positron

> <https://github.com/posit-dev/ark/issues/588>
> 
> * Author: @DavisVaughan
> * State: OPEN
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABwXx3RQ", name = "area: jupyter kernel", description = "", color = "C2E0C6")

Right now we always override `options(browse =)` and add our special Positron specific behavior. This isn't great for Jupyter kernels

https://github.com/posit-dev/ark/blob/738afbaa7984b592f8ab78bd714e04efe8bbf09c/crates/ark/src/browser.rs#L37-L59

From @lionel- 

> The right thing to do outside Positron is to use the default action, i.e. open in a web browser. So we should not set up that option unless Positron is connected. Ideally Positron would inject the option via its init file. Or the UI comm could launch an idle task to do it, but there would a short time where the option is not set with this approach.

## @jennybc at 2025-07-21T22:04:55Z

Specifically, in PR #877, here's the sort of error raised by calling `browseURL()` on a local folder's filepath:

```
~/tmp % jupyter console --kernel=ark
Jupyter console 6.6.3


R version 4.4.2 (2024-10-31) -- "Pile of Leaves"
Copyright (C) 2024 The R Foundation for Statistical Computing
Platform: aarch64-apple-darwin20

R is free software and comes with ABSOLUTELY NO WARRANTY.
You are welcome to redistribute it under certain conditions.
Type 'license()' or 'licence()' for distribution details.

  Natural language support but running in an English locale

R is a collaborative project with many contributors.
Type 'contributors()' for more information and
'citation()' on how to cite R or R packages in publications.

Type 'demo()' for some demos, 'help()' for on-line help, or
'help.start()' for an HTML browser interface to help.
Type 'q()' to quit R.


In [1]: browseURL("~/tmp")
  2025-07-21T21:35:02.025887Z ERROR  No help port is available to check if '~/tmp' is a help url. Is the help comm open?
    at crates/ark/src/interface.rs:2032

  2025-07-21T21:35:02.026935Z ERROR  Failed to browse url due to: UI comm not connected, can't run `open_with_system`.
    at crates/ark/src/browser.rs:22


In [2]: getOption("browser")
Out[2]:
function(url) {
    .ps.Call("ps_browse_url", as.character(url))
}
<environment: 0x11fd5bc70>
```

## @DavisVaughan at 2025-07-22T13:12:23Z

I think we need some guards in our (currently unconditional) `options(` setting calls that only set the options when we are in Positron. We will probably have to gather a number of `options(` calls that are specific to Positron features together and set them in one `if (is_position()) {` branch, and avoid them otherwise