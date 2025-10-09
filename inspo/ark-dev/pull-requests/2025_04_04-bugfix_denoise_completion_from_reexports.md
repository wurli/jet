# Suppress confusing logging messages re: namespace completions

> <https://github.com/posit-dev/ark/pull/765>
> 
> * Author: @jennybc
> * State: MERGED
> * Labels: 

Fixes #764 

There were a few ways to fix this. I picked an approach based on the root cause being an unconventional return value from `completion_item_from_symbol()`. It was returning `Option<anyhow::Result<CompletionItem>>`, which seems like it might just be plain odd? More objectively, it was the _only_ `completion_item_from_*()` function that did so.



