# Use `completion_item()` for keyword completions

> <https://github.com/posit-dev/ark/pull/308>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/2405

Thought this was addressed by https://github.com/posit-dev/amalthea/pull/290 but on a closer look it was a different issue.

Making a "simple" completion item with `CompletionItem::new_simple()` doesn't include the `data` field, which we require on the `completion_resolve()` side, otherwise we log an error and bail out. We should be using `completion_item()` as the base for all completion items, so that's what I've switched us to.

We didn't have a `CompletionData` variant for keywords, and nothing else really fit, so I went ahead and made one that is basically a no-op on the resolution side. This is fine, plenty of other variants are also no-ops.

