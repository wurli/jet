# Open files via Jupyter instead of LSP

> <https://github.com/posit-dev/ark/pull/357>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Follow-up to:

- https://github.com/posit-dev/amalthea/pull/193
- https://github.com/posit-dev/positron/issues/1885

As I was working on https://github.com/posit-dev/positron/issues/2999, I initially thought that `Backend` should be the sole owner of a `WorldState` struct which should not be clonable, which meant Backend could no longer be clonable. This PR is a step towards making the LSP the sole owner of `Backend` by changing our implementation of `file.edit()` to make a request via the UI Jupyter comm instead of the LSP. The Jupyter request is blocking, unlike the LSP one, but this is sufficiently fast that it shouldn't matter.

Note that I later figured out that a clonable state is a good thing as long it doesn't have any interior mutability. A clone of the worldstate is a _snapshot_ and it allows background workers to work without risking having their data change from under their feet. In any case, making the kernel independent from the LSP is cleaner (see posit-dev/positron#3180), so I've kept this commit.

