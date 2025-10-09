# Use a "function context" to create more responsive completions

> <https://github.com/posit-dev/ark/pull/819>
> 
> * Author: @jennybc
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/4441
Addresses https://github.com/posit-dev/positron/issues/2574
Addresses https://github.com/posit-dev/positron/issues/2417

The common theme of this trio of issues is that folks want the completion to be more responsive to text that is already there. We are only talking about function completions here. The overall goal is to make it easy to accept completion help when tinkering with existing text (editing the function name or retroactively adding a `pkg::` prefix), without getting annoyances like two function names or two sets of parentheses.

We had already dipped our toe in using information about the completion site with the notion of `parameter_hints`. That story started in #680 (but got revised in later refactoring #754).

This PR creates a new `function_context` field in `CompletionContext` that contains rich information on the completion site, for the purposes of function completion. It supersedes `parameter_hints`.

Main points of interest:

* New `FunctionContext` struct is "everything you ever wanted to know about the AST, when forming a function completion". New `function_context` field in  `CompletionContext`. New accessor/initializer `function_context()`.
  - It feels good to frame the problem in terms of `FunctionRefUsage` (call vs. value) and `ArgumentsStatus` (absent, empty, nonempty).
* New standalone test module in `lsp/completions/tests/function_completions.rs`. I did this because these function completion tests don't really belong in `completion_item.rs` (and also that will not scale gracefully if we start to test other types of completions similarly). Function completions come via (at least) three sources (search path, namespace, document), so the tests don't belong with the sources either.
  - The test utilities in `completions/tests/utils.rs` remove a lot of boilerplate and I'd like to do similar elsewhere.
* I use tricks to make it feel like you are accepting text that's already present. Consider the problem of adding a `pkg::` prefix after the fact:
    ```
    # this fails because build_articles() can't be found
    build_articles()

    # go back and add `pkgdown::`
    # completions pop-up when cursor is at @
    pkgdown::@build_articles()

    # Trick #1: hoist build_articles() to the top of the completion list
    # Trick #2: use a text edit to replace "build_articles(" with "build_articles(",
    #   which has the effect of just moving the cursor
    # Tab or return to accept existing text and put the cursor inside `()` and
    #   enjoy automatic parameter hints
    pkgdown::build_articles(@)
    ```

✨ I definitely used copilot for assistance, but I ruled it with a very heavy hand. The overall design is 100% human-crafted, but certain implementation details or using this syntax vs. that might come from copilot. I have certainly tried to implement in a way that is consistent with what I see elsewhere, but I'm happy to make changes.



