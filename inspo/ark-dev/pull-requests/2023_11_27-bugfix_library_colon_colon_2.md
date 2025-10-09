# Further refine / test `call_node_position_type()`

> <https://github.com/posit-dev/ark/pull/164>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

@lionel- this further refines your PR and merges into it. I was aware of this issue and planned to come back to it with a few additional ideas. It also affects call completions, i.e.

```r
vctrs::vec_sort(dire<tab>)
```

this currently doesn't provide `direction =` as a completion option due to the same problem (fixed with this PR).

---

It is also the root cause of https://github.com/posit-dev/positron/issues/127#issuecomment-1824045147

---

For the purpose of `call_node_position_type()`, I don't _think_ we need an `Ambiguous` variant. I think `fn(x<tab>)` can always be considered a `name` position for what this is used for. I think we can consider coming back to this in the future and adding that in if we need to do something special there.

I've gone through and removed the recursion from `call_node_position_type()`. I don't think we need it, as I don't think we actually need to look beyond the "previous leaf" to determine the position type. I think this makes it a little easier to think about all of the possible states, and works well for all of the new test cases I added (based on all the minor issues we've discovered).

## @lionel- at 2023-11-28T10:44:48Z

> I think fn(x<tab>) can always be considered a name position for what this is used for.

This makes sense with "names-heavy" UIs like dplyr or tidyr, but with more traditional programming we lean more towards passing values in the current environment positionally I think.

I wonder if we should make it a declaration to opt into one or the other? But I think the default should be to lean towards values when tabbing in compulsory arguments (ones before `...`). What do you think?