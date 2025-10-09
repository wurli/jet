# [Windows] `viewer` option can't open URL

> <https://github.com/posit-dev/ark/issues/790>
> 
> * Author: @cderv
> * State: CLOSED
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABSPDwBw", name = "bug", description = "Something isn't working", color = "E99695")

I believe this is the cause of the problem of 
* https://github.com/posit-dev/positron/issues/4843

which prevents `testthat::snapshot_review()` to work on windows. 

## Problem

ark sets `options(viewer =` in https://github.com/posit-dev/ark/blob/main/crates/ark/src/modules/positron/viewer.R

However, it does not seem to handle URL (i.e starting with `https`), but only valid file path on windows. 

This is because 
https://github.com/posit-dev/ark/blob/40908a3b2ba4c54bba05940ffd536444abf598a0/crates/ark/src/modules/positron/viewer.R#L15

On windows, `normalizePath` does not handle URL
```r
> normalizePath("http://127.0.0.1:7305", mustWork=FALSE)
[1] "c:\\Users\\chris\\Documents\\DEV_R\\quarto-r\\http:\\127.0.0.1:7305"
```

On linux
````r
> normalizePath("http://127.0.0.1:7305", mustWork=FALSE)
[1] "http://127.0.0.1:7305"
````

So this will use a wrong modified url to `utils::browseURL`
https://github.com/posit-dev/ark/blob/40908a3b2ba4c54bba05940ffd536444abf598a0/crates/ark/src/modules/positron/viewer.R#L28-L31

## Reprex: 

````r
# Run a server from R for the directory
> servr::httd()
To stop the server, run servr::daemon_stop(1) or restart your R session
Serving the directory C:\Users\chris\Documents\DEV_R\quarto-r at http://127.0.0.1:4321

# Set up some trace to see the input
> trace(utils::browseURL, 
      tracer = quote({
        cat(format(Sys.time()), "| browseURL called with URL:", url, "\n")
      }),
      print = FALSE)

# Use viewer option from ark https://github.com/posit-dev/ark/blob/main/crates/ark/src/modules/positron/viewer.R
> getOption("viewer")("http://127.0.0.1:4321")
2025-05-05 18:21:57 | browseURL called with URL: c:\Users\chris\Documents\DEV_R\quarto-r\http:\127.0.0.1:4321
````




## @cderv at 2025-06-05T18:29:53Z

I believe this is closed by 
- https://github.com/posit-dev/ark/pull/818

## @cderv at 2025-06-05T18:33:35Z

Yes checked with 
````
Positron Version: 2025.07.0 (user setup) build 49
Code - OSS Version: 1.100.0
Commit: 8b3e95ab7926c0643351147fdbcc1145ab8f5510
Date: 2025-06-05T03:51:08.416Z
Electron: 34.5.1
Chromium: 132.0.6834.210
Node.js: 20.19.0
V8: 13.2.152.41-electron.0
OS: Windows_NT x64 10.0.26100
````