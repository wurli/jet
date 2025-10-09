# Use Ark URIs for fallback sources in the debugger

> <https://github.com/posit-dev/ark/pull/852>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/2689.

Before this PR, fallback sources, i.e. generated sources for functions that don't have source references, were handled via the DAP protocol. Because of this we didn't have any control over what kind of editor would be opened on the frontend side and what file extension it would use. This caused these fallback editors to not behave as R files:

- No syntax highlighting
- No Positron-R features such as evaluating with Cmd+Enter

With this PR, we now open virtual documents managed by Ark when the debug session starts or updates. The DAP frames now point to these documents via URIs instead of the DAP "source references".

The URIs have the following scheme: `ark:ark-*pid*/debug/*session-id*/*source-hash*/*source-name*.R`

- The session ID disambiguates between debug sessions, though on hindsight I don't think we actually need it.
- The source-hash disambiguates between frames that have the same source name but different source code.
- The source name is the frame call.

These URIs remain valid for the duration of the debugging session, just like with DAP vdocs.


### QA Notes

Executing the following with Cmd+Enter (sourcing will have different behaviour) should take you to a virtual document (see also notes about vdocs in https://github.com/posit-dev/ark/pull/848).

Stepping in with F11 should take you to different vdocs.

In these documents you should see syntax highlighting and be able to evaluate code with Cmd+Enter.

```r
f <- function() {
    browser()
    g()
}

g <- function() {
    h()
}

h <- function() {
    browser()
    1
    2
    3
}

f()
```

I don't have any tests but I have verified via logs that we clean up these documents on session exit.


