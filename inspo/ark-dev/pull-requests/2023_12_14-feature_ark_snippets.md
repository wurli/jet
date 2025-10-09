# Implement snippet support in ark

> <https://github.com/posit-dev/ark/pull/183>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Paired with https://github.com/posit-dev/positron/pull/1948

Addresses https://github.com/posit-dev/positron/issues/1648#issuecomment-1828790832
Addresses https://github.com/posit-dev/positron/issues/1803

Letting Positron itself manage the snippets is actually kind of annoying because they show up very aggressively, often in places that aren't reasonable to have snippets.

Moving them down into ark means that we get (basically) full control over when to show the snippets. We now only show them in the "composite" case when we detect that the user is typing some kind of generic `identifier`.

In theory I think this also means they should show up if ark is used as a standalone jupyter kernel?

There are two cases in particular that make this worth it. File path completions and function argument completions, shown below:


https://github.com/posit-dev/amalthea/assets/19150088/40a34f94-968e-4309-b538-08fa447c3b44


https://github.com/posit-dev/amalthea/assets/19150088/7a551991-a515-43d0-a919-97642c6e7389

Implementation wise, this uses a few cool features:
- Rust embed to sure that `r.code-snippets` is available no matter if we are in a dev or release environment
- `Once` to ensure that we load the snippets in once, making access basically free (cost of a vector copy)

