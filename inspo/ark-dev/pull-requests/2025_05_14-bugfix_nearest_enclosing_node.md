# Use smallest spanning node as the "starter" node in completions

> <https://github.com/posit-dev/ark/pull/805>
> 
> * Author: @jennybc
> * State: MERGED
> * Labels: 

Fixes #778 

When we create a new `DocumentContext`, one of the pieces of data reflects some notion of the current node in the AST. Previously this was constructed as `ast.root_node().find_closest_node_to_point()` and I propose that we (mostly) switch to `ast.root_node().find_smallest_spanning_node()` instead. In this PR, I:

* Retain the previous notion of the current node under the name `closest_node`
* Recast the `node` field to mean "the node that point is actually in". (Frankly this is still somewhat aspirational, because so much existing code expects to start from something like a `"="` node, but this is a start. See #808 for more.)

This new definition of `node` is more favorable for completions (next I'll take #772 out of draft form, which will address #770), which is certainly the biggest user of `DocumentContext`, in terms of downstream code.

(The fact that redefining `node` has such a small impact on *anything else* below `crates/ark/src/lsp/` is sort of telling. It feels like there's a lot of bespoke fiddling around with the AST and nodes tucked away in various corners that could potentially be centralized/deduplicated over time. But not today.)

## @jennybc at 2025-05-21T22:31:53Z

OK @DavisVaughan I think this is ready if you want to give it one last look. These are the highlights of what's going on:

* Take a *partial* step in the direction of starting completions at the "node that contains the cursor", not necessarily the "closest node". For reasons discussed elsewhere and in #808, this is a complicated issue and you can't make changes in one place w/o making changes in other places. Hence the partial step.
  - In particular, when asking for completions on an empty line, the associated node is now the root 'Program' node. This is how this whole effort got started 😅 
* New field in `CompletionContext` that lets call, custom, and pipe completions access a shared notion of the "call node that contains point". Field is `containing_call_cell` and accessor is `containing_call_node()`.
  - Relies on a new function `node_find_containing_call()`.
* Adopt `point_from_cursor()` in any test module that I touched. Introduce other test helpers when it's easy to do so.
* Add many tests around completions in:
  - multiline contexts
  - the "value" position, in the sense of "name = value"