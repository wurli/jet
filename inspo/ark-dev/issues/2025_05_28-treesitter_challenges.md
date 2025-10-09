# Treesitter challenges

> <https://github.com/posit-dev/ark/issues/808>
> 
> * Author: @jennybc
> * State: OPEN
> * Labels: list(id = "LA_kwDOJkuGPc8AAAABwXx7Aw", name = "area: language server", description = "", color = "C2E0C6")

*My recent foray into the language server and, more specifically, completions has given me Some Opinions™️, which I discuss with @lionel- and @davisvaughan. We agreed I'd capture some of this friction I'm noticing as a newcomer to the codebase. This issue is inspired by work on #778 and #805.*

As I get to know the completions codebase, I've been surprised at the widespread, very low-level interaction with the treesitter syntax tree. In hindsight, it's clear I expected activities to be framed in terms of nodes of type, e.g., "call", "arguments", or "argument". I didn't expect so much logic around anonymous nodes, such as `"("`, `")"`, `","`, and `"="`.

To a considerable extent, you cannot avoid working with these low-level nodes. I mean, someone has to do it! Therefore one coping strategy is to keep building out a nice layer of well-tested wrapping around treesitter and to increase usage of this wrapping (i.e. try to eliminate bespoke low-level tree handling in functions that do high-level tasks).

But it may also be true that this is a legitimate downside of using the treesitter tree or parser directly (or at all?). This issue is a place to record challenges that come from treesitter.

### Whitespace is hard

The fact that whitespace is basically not accounted for in the syntax tree is quite painful, because the cursor quite often has whitespace on one or both sides. In the language server, we constantly need to determine which node is "most associated" with the cursor. It's accurate-ish to say that treesitter's treatment of whitespace makes the "most associated" node almost undefined in these cases. It certainly puts you in a gray area.

Let's look at an example! Consider this code, where `@` indicates the cursor:

```
options(
  a = @
)
```

Here's treesitter's view of that code. On the left, I overlay treesitter coordinates and on the right is the resulting syntax tree.

```
    0  1  2  3  4  5  6  7  8
    ┌──┬──┬──┬──┬──┬──┬──┬──┐
 0  │ o│ p│ t│ i│ o│ n│ s│ (│
    └──┴──┴──┴──┴──┴──┴──┴──┘

    0  1  2  3  4  5  6            program [0, 0] - [3, 0]
    ┌──┬──┬──┬──┬──┬──┐              call [0, 0] - [2, 1]
 1  │  │  │ a│  │ =│  │                function: identifier [0, 0] - [0, 7]
    └──┴──┴──┴──┴──┴──┘                arguments: arguments [0, 7] - [2, 1]
                                         open: ( [0, 7] - [0, 8]
    0  1                                 argument: argument [1, 2] - [1, 5]
    ┌──┐                                   name: identifier [1, 2] - [1, 3]
 2  │ )│                                   = [1, 4] - [1, 5]
    └──┘                                 close: ) [2, 0] - [2, 1]
```

To paraphrase the treesitter docs about [i, j] coordinates:

> The row number i gives the number of newlines before a given position.
> The column j gives the number of characters between the position and beginning of the line.

(It's really bytes, not characters, but that's not important for this discussion.)

A treesitter position:

* Sits ON a line
* Sits BETWEEN two characters

In the example, the cursor `@` is at position [1, 6].
So which node is the cursor "in"?
If your job is to provide completions, which bit of syntax are you helping the user to fill in?

IMO there are two reasonable answers. You're either in:

* An "argument" node. Here, the node with text `a =`.
* The potential "value" node that *could* exist as a child of the "argument" node.

I view these as equivalent, because if you chose option 1, you would then have logic to bring yourself to option 2. That's just a matter of how you design the interface.

If you use bare treesitter tooling, here's the node you are "in":

* The "arguments" node, which is everything between the `"("` and the `")"`.

You can read this off the tree, because the "arguments" node is the smallest node with a span that contains position [1, 6].
Selecting the "arguments" node is very unfavorable for providing completions, though. It's too high.

What if there was no space between the `"="` and the cursor?

```
options(
  a =@
)
```

Bare treesitter tooling would _still_ say the cursor is in the "arguments" node.
Ark already has some wrappers around treesitter where we have (somewhat) fixed this up.
`find_closest_node_to_point()` would latch on to the `"="` in this case.
(And quite a bit of existing logic expects to solve problems in this "bottom up" way, although I'm not sure it *has* to be this way.)

This is a good place to record the capturing behaviour at node boundaries.
In treesitter, a node span is sticky / inclusive on the left and not sticky / exclusive on the right.
Concretely, where `@` indicates the cursor position and `[ ... ]` indicates a node's span:

```
? ? @[ ... ]  ? ? <- the cursor IS in the node
? ?  [ ... ]@ ? ? <- the cursor IS NOT in the node
```

`find_closest_node_to_point()` would say the cursor is in the node in both cases.

## Executive summary

The treatment of whitespace makes treesitter syntax trees tricky to use directly for language server tasks.
You generally need to walk up and/or down to identify the node that really drives your actions.

It feels like ark's language server currently has these tricky gymnastics inlined throughout the codebase.
In the future, it would be nice to give ourselves a more ergonomic interface to the tree.


## @DavisVaughan at 2025-05-22T14:20:33Z

Random musing - it's possible that what we actually want would be broken into two passes

Pass 1 - A helper for "Am I somewhere in an Arguments node?", which would return `true` for all of your examples above, regardless of the whitespace issue. We'd also have a way to gain access to that Arguments node we are inside. This _always_ spans the user's cursor.

Pass 2 - Now that we know we are inside an Arguments node, we need to find the child of that Arguments node that directly precedes the user's cursor. This _may not necessarily_ span the user's cursor (like the `=` with the example above).

---

Pass 1 is roughly `find_smallest_spanning_node()` if you adjusted it to be `find_smallest_spanning_arguments_node()`.

Pass 2 is roughly `find_closest_node_to_point()`, where you'd start from the Arguments node returned from pass 1 and dig into it to find the closest child node preceding the cursor.

---

We could likely end up with various flavors of Pass 1 helpers for `find_smallest_spanning_<THING>_node()` (or we could probably parameterize a single function on a `NodeKind` to avoid a proliferation of helpers), depending on what we consider to be the start point for a particular feature's analysis.

I'm somewhat convinced that each feature (different types of completions, hover, signature, etc) has _its own specific set of requirements_ about the exact node you care about starting from. So while it has proven useful for us to cache a `node` in `DocumentContext` so far, it is possible that it will be more useful for us to have targeted helpers like these that we can use instead, because not everyone will want to use the same `node` (some features will just care about the smallest spanning node, some will care about the closest preceding node, etc).

---

Another way of thinking about this is that tree-sitter isn't inherently broken or anything. We likely just have the wrong set of abstractions built on top of it for our required tasks. But now that we have a bunch of examples of the tasks we'd like to do, we can probably build better tree-sitter tooling. 

## @jennybc at 2025-05-22T15:32:07Z

Great summary:

> Another way of thinking about this is that tree-sitter isn't inherently broken or anything. We likely just have the wrong set of abstractions built on top of it for our required tasks. But now that we have a bunch of examples of the tasks we'd like to do, we can probably build better tree-sitter tooling.

## @lionel- at 2025-05-28T09:34:55Z

I had a feeling that somehow the direction from which we look from the cursor is important and that part of our struggles is how "closest node at point" is insensitive to direction.

I reviewed approaches in Ruff and Rust-Analyzer and found interesting patterns:

- They both start by converting the cursor position to a token. This token represents a single token in the general case but can also represent _two of them_ when the cursor sits between two tokens.

  Note that in both RA and Ruff the token may be a trivia (e.g. comment or newline). Even though Ruff discards trivia token from the parse tree (as Biome and Air do), they still store the full token vector and that's what they use to match the cursor position to.

- There are two approaches for picking a token. The most common is to rank them. Trivia tokens are given a rank of 0. The ranking generally give higher precedence to identifiers over punctuation (unnamed nodes in TS jargon). You can see the ranking in action at:

  -  https://github.com/rust-lang/rust-analyzer/blob/a420ef2b1791d6ebce0de63a6b561f1a5721bf21/crates/ide/src/hover.rs#L166-L181
  - https://github.com/rust-lang/rust-analyzer/blob/a420ef2b1791d6ebce0de63a6b561f1a5721bf21/crates/ide/src/goto_definition.rs#L47-L62
  - https://github.com/astral-sh/ruff/blob/66ba1d87751eac470475156e5060e0d983978966/crates/ty_ide/src/goto.rs#L187-L197

- The other approach is to give a _direction_. I haven't seen this used in Ruff but RA uses it for completions (Ruff doesn't complete based on cursor yet).

  The other very interesting thing to note about completions in RA is that they first start by inserting a marker, a sentinel string at the cursor position. The resulting file is called "speculative". They do this for expansions but also to "fix" the parse tree in case the user hasn't typed anything.

  Then they pick the token always on the right from the offset to make sure they match the identifier (which looks like `@MARKER` if nothing was typed, `foo@MARKER` if user typed `foo`, or `foo@MARKERbar` if cursor is in the middle of a word: https://github.com/rust-lang/rust-analyzer/blob/7fa66d67a72bcad236b50907e14f7464c47ecede/crates/ide-completion/src/context/analysis.rs#L58-L60

  They then match the construct with an interesting `loop break val` pattern, iterating over parents:
  https://github.com/rust-lang/rust-analyzer/blob/a420ef2b1791d6ebce0de63a6b561f1a5721bf21/crates/ide-completion/src/context/analysis.rs#L603. This part is to figure out the _semantic context_ of the speculative token.

So my general feeling regarding actions to take:

- Always use the same process in all handlers, just like they do in Rust-Analyzer and Ruff: convert the cursor position to a token (either via ranking or matching directionally), then match constructs to the token ancestry. We don't necessarily need more abstractions (besides typed node ctors or a collection of predicates), maybe we just need a consistent practice.

- Look into speculative insertions for completions. This would solve the "cursor in whitespace" issue by ensuring we never start from trivia or punctuation, but instead from an identifier-like token.
