# Refactor some infrastructure for completions

> <https://github.com/posit-dev/ark/issues/681>
> 
> * Author: @jennybc
> * State: CLOSED
> * Labels: 

In https://github.com/posit-dev/positron/issues/1818, @DavisVaughan says:

> It's probably time to introduce a `CompletionOption` struct to contain something like `in_help: bool` and `promise_strategy: Option<PromiseStrategy>` which gets fed through all of the completion functions as an immutable reference.

In discussions around #680, we've reached consensus that it is definitely time to do this. The details will likely be somewhat different than what's sketched above and we'll discuss that here.

**Why is this needed?** Many of the low-level helpers that provide completions from a certain source (from the search path, from the workspace, from a call, and so on) actually need some syntactic and/or semantic information about the completion site.

For example, #680 addresses https://github.com/posit-dev/positron/issues/1818 and https://github.com/posit-dev/positron/issues/2338, where we want to control the automatic addition of parentheses when completing a function. If we're helping the user write a function invocation, we want parentheses. We want to leave the cursor inside these parentheses and trigger the display of parameters hints. But if we're completing the function name in a function value position (e.g. it's the subject of `debug()` or `?`), we don't want parentheses.

The solution in #680 is (fondly) described as "dumb" because a specific assessment is made about the completion site very early, in `provide_completions()` and then threaded through many, many layers of helpers as the boolean `no_trailing_parens`. It would be nice to encapsulate this whole matter inside `completion_item_from_function()`, but there are (at least) two problems with that:

1. The document context is required to analyze the completion site and is not available to helpers like `completion_item_from_function()`. Such an approach would just require threading _that_ through to lots of new places. (See #369 for a sense of how that plays out.)
2. `completion_item_from_function()` gets called once per function-that-is-in-scope, which is >2300 even in a vanilla R session. But this assessment of the completion site only needs to happen **once**, at most.

Several more completion issues are coming down the pike (https://github.com/posit-dev/positron/issues/4441, https://github.com/posit-dev/positron/issues/2574, https://github.com/posit-dev/positron/issues/2417, for starters) which are likely to play out in a similar fashion. The existing logic for determining if we are inside a pipe, finding that pipe's root node, and behaving appropriately is also related.

**TL;DR the type of solution used in #680 is not going to scale gracefully as we continue to work through the long list of completion improvements (https://github.com/posit-dev/positron/issues/1603).**

One more note from discussion of #680:

The current pattern of "append new entries to a growing list of `CompletionItem`s, then filter to unique items at the end" isn't very favorable for development (seen, for example, in [`completions_from_composite_sources()`](https://github.com/posit-dev/ark/blob/846b9e73d0b5ad336af4b11fb12ba60ab4fc22b6/crates/ark/src/lsp/completions/sources/composite.rs)). Many of our completion items are nominated through more than one helper and it can be useful to understand which one(s) contributed a specific `CompletionItem`. It would be nice to assemble this collection in a way that preserves provenance. We also de-duplicate looking only at the `label` but two items could match on that and differ in other fields, such as the actual `insert_text` (!).


