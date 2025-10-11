# Ensure we show completions for anonymous nodes

> <https://github.com/posit-dev/ark/pull/199>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/1803#issuecomment-1865020221

`if`, `for`, and `while` are snippet keywords that _also_ happen to exactly match the names of anonymous nodes in the tree-sitter grammar. This means their "kind" comes through as their corresponding name (i.e. like `"if"`).

Typically an `"if"` "kind" is reserved for "we detected a full if statement", but in our grammar we end up with both _named_ and _unnamed_ (anonymous) nodes for `"if"`. The named node corresponds to a full if statement, the unnamed node corresponds to a literal `"if"`. I tried to be careful and only return completions in the unnamed case. We've had to do something similar elsewhere.

The `if:` generates the named node, the `"if"` generates the unnamed node:
https://github.com/r-lib/tree-sitter-r/blob/2402f051a9605416f2d61eb82740fa0773c35d05/grammar.js#L223-L224

