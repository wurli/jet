# Add completions for lazy data exports

> <https://github.com/posit-dev/ark/pull/5>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/rstudio/positron/issues/599

You'll notice that this forces the lazydata promises. This is due to `completion_item_from_object()`, where we try to evaluate promises. I don't think we should evaluate them if they haven't been resolved yet, but that is a change for another PR.

The little hiccup you see after I type `nycflights13::` is me being impatient. It takes a second or two to load the package to then be able to show the completions. It does work if you just hang out for a second. It does take noticeably longer than in RStudio though, so maybe worth looking into.

I can't seem to figure out where the help documentation gets pulled in. i.e. `dplyr::across` will pop up some help docs for across, but where do we do that?

So this PR is ready to merge with the caveats of:
- Lazy data bindings are currently forced, but I'll look into that
- There is no help documentation that pops up for the lazy objects, but I can look after that after asking a few questions about it internally

https://github.com/posit-dev/amalthea/assets/19150088/1a3721f5-6899-4a42-a650-610de4d62b7e



