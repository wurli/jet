# Support functions in `View()`

> <https://github.com/posit-dev/ark/pull/848>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses posit-dev/positron#2945.

This PR adds support for functions in `View()`.

- For functions with valid source references, `View()` jumps to the function location in the source file.

- For functions in packages that don't have source references, `View()` first generates a virtual namespace document (the same as for debugging) and jumps into that file.

  Note we only generate the virtual namespace source if `View()` is called at top-level. The generation process causes global mutation of the namespace objects to attach the source refs, and we can't do that if code is already running as it might make assumptions about the srcrefs.

- As a fallback, we deparse the function and generate a virtual document for it.

Currently the virtual documents in the fallback approach are never cleaned up.  I'm not too worried about the memory leaks that this causes but we would ideally do better. The `TextDocumentContentProvider` API doesn't have any facilities for clean up but we could in principle open an editor for the vdoc, then watch over editors and trigger clean up when the editor is closed. However that would require lots of communication between the frontend and the backend and being careful about synchronisation. So I think this would be better handled entirely on the frontend side. This can be done as follow-up work.

I also made a small change to the Ark URI scheme to disambiguate multiple sessions. The URIs now contain the process ID of the Ark that generated them. If for some reason the frontend tries to open a vdoc for a session that's now in the background, you'll get an invalid vdoc editor. With the frontend-side PR, this sort of errors are now more user friendly. The error is mentioned in the virtual editor and the error message is logged.

All this is mostly defensive, it would be hard to trigger this error. But since the vdocs are currently provided by the LSP, and since the LSP is now transient, I just wanted to make sure we dealt with this more gracefully. In principle the vdocs should be provided via Jupyter instead of LSP and we'd dispatch to the relevant session based on an URI session parameter. It's not a priority to do it that way for now though.


### QA Notes

I've added some backend tests but there's some subtle behaviour and interactions on the frontend side that's not tested.

When viewing a function from a package, a virtual document containing the entire package should be generated and `View()` will take you at the appropriate location in that document. For instance with `lm()` from the stats package:

```r
View(lm)
```

Note that this is the same file that is used when you `debug()` a function in that package. The two features must not interfere with each other, i.e. you can do the following before and after `View(lm)` and everything should still work:

```r
debug(lm)
lm()
undebug(lm)
```

For functions with valid source references, `View()` should take you to the source file. This should be a regular editor that can be edited, unlike the virtual namespace file. Here's an example with a tempfile:

```r
# Source file with a function definition for `foo`
file <- tempfile(fileext = "R")
cat("foo <- function() 1", file = file)
source(file)

# Now view it
View(foo)
```

For functions that don't have source references, for instance script functions evaluated in the global environment, a virtual document is opened with the deparsed source:

```r
foo <- function() 1
View(foo)
```

You can see the URI for this document by hovering over the editor tab. The URI contains the Ark PID for disambiguation, as well as the environment name if it has one (in this case, "global"). The filename is based on the identifier provided to `View()` (in this case, "foo.R").

When the environment in which `View()` is called doesn't have a name, the URI contains the environment's address instead of a name:

```r
# Local function -> URL includes environment address
local({
    foo <- function() 1
    View(foo)
})
```

When `View()` is called with an expression instead of an identifier, the file is "unknown.R". Note that the environment address is shown instead of the name, due to a limitation of the current implementation:

```r
# Non-named function -> unknown.R
View(identity(foo))

# Same
xs <- list(foo = foo)
View(xs$foo)
```

A related change made to this PR is that trying to open an Ark URI that doesn't exists is now more user friendly. We no longer open an error dialog, instead an editor is opened with an error message inside and an error is logged in the extension output.

```r
# No longer opens an error dialog
.rs.api.navigateToFile("ark:hello")
```

Finally, we've also made functions viewable from the variable pane. After running the examples above you should see `foo` and `xs` defined. You can view the source by clicking the view button that has now appeared in the Variables pane:

<img width="769" alt="Screenshot 2025-06-25 at 10 36 05" src="https://github.com/user-attachments/assets/7852e37a-2c13-4e66-9b00-04d09109a28a" />

I don't have any backend tests for this feature.


## @jmcphers at 2025-06-24T17:56:51Z

Oh, one other thing that could be improved -- if you `trace()` a function then it turns it into a `functionWithTrace` (common when setting breakpoints), and if you try to `View()` that you get a dump of the S4 object. I think that should show the original function body instead.

<img width="1133" alt="image" src="https://github.com/user-attachments/assets/0e73a54c-46db-4a63-bc7f-3bfbf7098682" />


## @jennybc at 2025-06-25T01:36:13Z

I have looked over the source changes, but I'm going to focus on reporting my experience using the PR. I'm not sure why I'm having a different experience than @jmcphers or if we're exercising it differently. I've got a dev build of Positron up, from https://github.com/posit-dev/positron/pull/8262 and a local ark build from this PR. To confirm:

```
> .ps.ark.version()
                                 branch                                  commit                                    date 
                "feature/view-function"                              "df8a918f"               "2025-06-24 17:01:56 PDT" 
                                 flavor                                    path                                 version 
                                "debug" "/Users/jenny/rrr/ark/target/debug/ark"                               "0.1.192"
```

*I'm going to edit this comment or make new ones as I walk through the suggested usage.*

## Virtual namespace document

First I do:

```
debug(data.frame)
data.frame()
undebug(data.frame)
```

I can walk through `data.frame()` in the virtual namespace document and "continue" to exit the function. When I do `undebug()`, I find myself back in the debugger 🤔 

```
...
Browse[1]> n
debug at ark:ark-33066/namespace/base.R#6165: mirn <- missing(row.names)
Browse[1]> n
debug at ark:ark-33066/namespace/base.R#6166: mrn <- is.null(row.names)
Browse[1]> c
exiting from: data.frame()
> undebug(data.frame)
Browse[1]>
```

from which I then cannot escape. I.e. `Q` does not work. In Positron 2025.07.0 (Universal) build 155 I also have a somewhat odd experience with the same code, with the debugger popping back up as soon as I exit `data.frame()`. So I can't disentangle existing odd behaviour I'm seeing on `main`, so to speak, from this PR.

## @jennybc at 2025-06-25T02:09:02Z

## Source refs exist

I've installed usethis with source refs and this appears to be working well ✅ I can do `View(usethis::proj_get)` and I'm taken to a preview of `/Users/jenny/rrr/usethis/R/proj.R`.

The example with a tempfile also works as advertised. *Sidenote: should be `tempfile(fileext = ".R")` with a dot. At first I thought there was a problem with filename handling in the PR, then I realized it was just this.*

## Source refs don't exist

```
foo <- function() 1
View(foo)
```

The virtual document with deparsed source is created with the expected content and URI. 

Likewise when `View()` is called with an expression.

```
# Non-named function -> unknown.R
View(identity(foo))

# Same
xs <- list(foo = foo)
View(xs$foo)
```

These URIs ☝️ contain the environment's address. I *think* I expected these to be "global" 🤔 in this case?

The `local({})` example works as promised.

## @lionel- at 2025-06-25T06:49:34Z

> I can walk through data.frame() in the virtual namespace document and "continue" to exit the function. When I do undebug(), I find myself back in the debugger 

I got in this state from time to time, from what I can tell something in Ark is calling `data.frame()` and triggering the debugger, getting us in a bad state. It doesn't happen all the time for me though.

I'll update the QA examples to use a different, less risky function.

## @lionel- at 2025-06-25T07:01:50Z

> These URIs ☝️ contain the environment's address. I think I expected these to be "global" 🤔 in this case?

That's a reasonable expectation! The thing is that we get `env` and `var` from the dispatching function, and it only provides those if `var` is a valid object in `env`. That's because these are used for live-updating of data frames in the data viewer (and could be used for live updating of functions in a vdoc editor).

I could have the dispatching function pass those even when partially invalid, along with a boolean indicating whether the binding is valid. I'd prefer not to touch the data frame code path at this time though.

## @lionel- at 2025-06-25T09:18:58Z

@jmcphers Both requested features now implemented!

Regarding the variable pane, I'm returning `viewerId: null` from the `view` request. I had to update the comm contract in the frontend PR to make the result of `view` optional. Otherwise I'd have to return `viewerId: ""` and count on the fact that this value is treated as falsy here: https://github.com/posit-dev/positron/blob/88b998e5bb2bd84ee1e337f5f96b4ec44b90489f/src/vs/workbench/contrib/positronVariables/browser/components/variableItem.tsx#L141-L143. I preferred explicitness here.