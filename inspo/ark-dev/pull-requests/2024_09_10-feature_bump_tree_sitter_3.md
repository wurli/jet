# Bump tree-sitter-r 3 - New `"open"`/`"close"` fields, can use `node.is_missing()`, no more `..i` diagnostic

> <https://github.com/posit-dev/ark/pull/517>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/3632

Includes these 11 commits from tree-sitter-r https://github.com/r-lib/tree-sitter-r/compare/9d1a68f8f239bc3749a481ac85e2163e24f6362c...63ee9b10de3b1e4dfaf40e36b45e9ae3c9ed8a4f

Three notable ones:
- https://github.com/r-lib/tree-sitter-r/pull/110 (greatly simplifies some existing code around this)
- https://github.com/r-lib/tree-sitter-r/pull/112 (allows us to actually use `node.is_missing()`, fixing one of our diagnostics)
- https://github.com/r-lib/tree-sitter-r/pull/118 (fixes `..i` diagnostic)

## @lionel- at 2024-09-20T13:46:13Z

I forgot these notes:

> If a syntax error spans > 20 lines, it is now truncated to only show a squiggle on the start_point of the range, and it states that this is a Syntax error. Starts here and ends on row {row}

I like it!

> we only consider running semantic diagnostics down a section of the tree if we know that section of the tree does not contain any syntax errors.

Seems reasonable, and solves the problem of false positives semantic diagnostics due to syntax errors.