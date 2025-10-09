# Bump tree-sitter-r 4 - Syntax vs Semantic Diagnostics

> <https://github.com/posit-dev/ark/pull/523>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/2943

This PR also contains https://github.com/posit-dev/ark/pull/529 which was merged into it

Pulls in a whopping 34 more commits from tree-sitter-r https://github.com/r-lib/tree-sitter-r/compare/63ee9b10de3b1e4dfaf40e36b45e9ae3c9ed8a4f...99bf614d9d7e6ac9c7445fa7dc54a590fcdf3ce0. It's really only 3 main changes. We pull in all 3 at once because they are a bit intermingled with each other.

- https://github.com/r-lib/tree-sitter-r/pull/92
    - Improves tree-sitter's error recovery by better allowing it to "recover" missing `)`, `}`, and `]` tokens
- https://github.com/r-lib/tree-sitter-r/pull/119
    - Helpful for targeting by field name 
- https://github.com/r-lib/tree-sitter-r/pull/132
    - Improves tree-sitter's error recovery by removing a massive amount of grammar ambiguity / states

With these 3 changes, I was able to greatly improve our diagnostics engine.

It has been split into two parts - a syntax path, and a semantic path:
- `diagnostics.rs` holds the entrypoint and the semantic path
- `diagnostics_syntactic.rs` holds the syntax path

In a future PR I'll do a mostly pure "rearrangement" PR to clean this structure up a bit more. I haven't done that yet to make it clear what has been moved out of `diagnostics.rs`.

## Syntax diagnostics

Diagnostics based purely on `ERROR` and `MISSING` nodes in the tree-sitter AST.

- `MISSING` nodes give us a nice way of detecting missing closing delimiters like `}` or `)` and seems to work nicely

- `ERROR` nodes allow us to report syntax errors in general. We don't get that much info from tree-sitter about exactly what went wrong, so I just report `Syntax error` most of the time.

A few more improvements in this realm:

- If a syntax error spans > 20 lines, it is now truncated to only show a squiggle on the `start_point` of the range, and it states that this is a `Syntax error. Starts here and ends on row {row}`. This helps us avoid overwhelming the user in the (very few!) cases where this can still happen. In my experience, mostly when they have an unmatched string delimiter like a stray `"` that causes everything after it to look like a weird unclosed string.

- `ERROR` nodes are no longer "recursively" reported. i.e. it is very common for an `ERROR` node in tree-sitter to have _children_ that are also `ERROR` nodes. Like what you see below. Previously we actually reported both `ERROR`s! This was particularly problematic because those outer `ERROR` nodes often span a huge number of lines, while the inner `ERROR` nodes can be very precise and target the exact problem quite nicely. We now only report what I call "terminal" `ERROR` nodes that don't have any `ERROR` children. This greatly improved that "aggressive diagnostic" issue where the whole file would light up in squiggles.

```r
#> ── Text ────────────────────────────────────────────────────────────────────────
#> 1 + }
#> 
#> ── S-Expression ────────────────────────────────────────────────────────────────
#> (program [(0, 0), (0, 5)]
#>   (float [(0, 0), (0, 1)])
#>   (ERROR [(0, 2), (0, 5)]
#>     "+" [(0, 2), (0, 3)]
#>     (ERROR [(0, 4), (0, 5)])
#>   )
#> )
```

## Semantic diagnostics

Semantic diagnostics are now run in a separate path from syntax diagnostics, this has the following really nice benefit - we only run semantic diagnostics on top level expressions (i.e. children of `root`) that `node.has_error()` returns `false` for. In other words, we only consider running semantic diagnostics down a section of the tree if we _know_ that section of the tree does not contain any syntax errors.

This actually works quite nicely in practice.
- Top level expressions with no syntax issues get semantic diagnostics
    - And, importantly, all the code in the semantic path can be written _knowing_ there aren't any potential `ERROR` nodes to be wary of 
- Top level expressions with syntax issues get syntax diagnostics
    - Once the user fixes the syntax issues, they get semantic diagnostics for that fixed chunk too 
- The whole file still gets a mix of both semantic and syntax diagnostics, depending on what could be parsed

## Improvement examples

This shows the "truncation" idea once a _syntax_ error spans >20 rows

https://github.com/user-attachments/assets/cb3d80cc-0712-4294-ac07-2f2b6fa0128d

This shows improvements in the example in https://github.com/posit-dev/positron/issues/2943, which used to light up the whole file

https://github.com/user-attachments/assets/bd6fab72-888b-492e-9289-f84c5b4ed124

This is a tree-sitter-r test file, with many syntax errors. Note that 1) it doesn't light up the whole file and 2) it still shows some semantic issues too (symbol not found errors)

https://github.com/user-attachments/assets/40cc5fca-e87d-49cd-9709-eb34a97181df

Improvements on the example from https://github.com/posit-dev/positron/discussions/4177. We often target the missing opening/closing node now. At the very end it shows that `}` doesn't have a matching opening `{`, and I do think that is still a _technically_ correct syntax error message, even if we'd like to show the unmatched `(` (Positron does at least highlight that unmatched `(` in red here, which is nice)

https://github.com/user-attachments/assets/b622440a-dc48-47ba-94a0-627d7815fe28



