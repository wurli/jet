# Stop excluding base::return() and base::function from completions

> <https://github.com/posit-dev/ark/pull/768>
> 
> * Author: @jennybc
> * State: MERGED
> * Labels: 

I'm opening Completion Month with a real banger! Let's stop explicitly excluding `base::return()` and `base::function` from completions.

Relates to https://github.com/posit-dev/positron/issues/4842

I've

* Thought about this 🤔 
* Looked through relevant comments and `git blame` in ark
* Looked at what RStudio does

and I can't find any reason to _not_ just let `base::return()` and `base::function` show up in our completions. They will both be contributed by our search path completions, coming from `base`.

Both are represented by snippets, which I have some mixed feelings about. So I plan to discuss some additional improvements relating to the snippet treatment of `function` and `return()`. We could also discuss https://github.com/posit-dev/positron/issues/1850 at the same time. But perhaps this simple fix can happen first, especially for `return()`.

## @kevinushey at 2025-04-09T16:33:16Z

Does Ark provide the appropriate escaping so that the completion from `base::` gives `` base::`function` ``?

## @jennybc at 2025-04-09T17:38:31Z

@kevinushey No. That is already on my radar and will appear soon in its own issue or added to an existing issue. I definitely bumped into this while playing around for this PR.