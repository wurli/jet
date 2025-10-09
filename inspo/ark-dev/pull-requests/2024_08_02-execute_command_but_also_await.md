# Add new version of "execute a command" that also waits

> <https://github.com/posit-dev/ark/pull/460>
> 
> * Author: @juliasilge
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/3256

You _can_ wait for commands in some situations to do something vs. only kick them off. An example is `'workbench.action.files.saveFiles'` which has an async accessor that waits through `editorService.saveAll()` and then returns success. This PR adds support for treating a command as a frontend request rather than an event. It basically adds back functionality that I took out in https://github.com/posit-dev/ark/pull/267.

I currently tend to like this idea of having both an event version (doesn't wait) and a method version (does wait) but I guess I could be convinced that they all should be methods, like I had them originally, but that wait?

## @juliasilge at 2024-08-02T23:19:47Z

Related to https://github.com/posit-dev/positron/pull/4228

## @juliasilge at 2024-08-06T20:05:34Z

As discussed in https://github.com/posit-dev/positron/pull/4228, this now just has one "execute a command" RPC; it is a request and it waits for the command to be done.

I looked at [our current use of `.ps.ui.executeCommand()`](https://github.com/search?q=repo:posit-dev/ark%20%22.ps.ui.executeCommand%22&type=code) and I believe it is appropriate to _always_ wait for what we have so far; this is also consistent with RStudio behavior. I don't think we need to spend time right now setting up two versions in ark, one that waits and one that is "fire and forget". We can continue to pay attention to this, and see if we will need it in the future.