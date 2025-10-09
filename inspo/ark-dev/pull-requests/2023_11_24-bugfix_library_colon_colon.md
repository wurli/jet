# Fix `library()` completions when user has typed prefix

> <https://github.com/posit-dev/ark/pull/161>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses posit-dev/positron#1887.
Addresses https://github.com/posit-dev/positron/issues/127#issuecomment-1824045147

After the recent update to completions this is no longer handled by the `library()` handler:

```r
library(dpl<>)
```

That's because of the way arguments categorised as "Other" by `call_node_position_type()` are treated. The previous implementation used "value" as default position type and passed that to the `library()` completion handler. This is no longer the case which causes the handler to be ignored when the user has typed some characters.

The new implementation has improved detection of positions outside of delimiters but these are subsumed in the "Other" category. To fix this I added an enum value "Outside" to untangle outside positions and ambiguous positions. And we now treat the latter as "value", as before.

Maybe the category for ambiguous positions should be renamed from "Other" to "Ambiguous"? At least that's how I interpret this category. We treat `foo(<>)` as name, `foo(bar = <>)` as value, and `foo(bar<>)` could be either?

