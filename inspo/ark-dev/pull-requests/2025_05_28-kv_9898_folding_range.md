# Add folding ranges

> <https://github.com/posit-dev/ark/pull/815>
>
> * Author: @kv9898
> * State: MERGED
> * Labels:

Moving back from https://github.com/posit-dev/air/pull/146.

Old description:

It seems that the call for folding ranges has been quite a while: https://github.com/posit-dev/positron/issues/18, https://github.com/posit-dev/positron/issues/2924, https://github.com/posit-dev/positron/issues/3822.

I was initially only thinking about adding foldable comments, but it seems that doing this will disable the existing folding support for things like regions and brackets. Therefore, I rewrote these functionalities also.

The PR already supports the folding-range-handling of brackets, regions, code cells, indentations and *nested* comment sections:

(The screenshot is new, though)
![image](https://github.com/user-attachments/assets/19ed3e13-47ad-48d0-977d-5d2a722fa9f2)

Note that, compared to the [old PR](https://github.com/posit-dev/ark/pull/615), this PR no longer relies on a naive search for brackets. Instead it uses the new `tree_sitter` of air to walk down the AST for node handling.


## @kv9898 at 2025-06-03T16:05:10Z

are we good to go?

## @lionel- at 2025-06-04T09:07:46Z

Let's try this out.

When this is ported to Rowan we should also consider a simple loop over an iterator that walks each node in DFS, like this: https://github.com/rust-lang/rust-analyzer/blob/789d9153e4615c63027305299f3a33d933e96464/crates/ide/src/folding_ranges.rs#L52

For nodes you can easily find the ending, and for sections we'd just match the next closing section within the current node. This way we avoid recursing (slower due to function calls, potential for overflow) and we simplify the logic (for instance no need to maintain stacks).

Thanks for the contribution!

## @DavisVaughan at 2025-06-04T13:12:26Z

Thanks for playing ping pong with us @kv9898, sorry again about that!
