# Bump tree-sitter-r 5 - Leading newline skipping, tree-sitter 0.23.0 update

> <https://github.com/posit-dev/ark/pull/529>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Contains these commits https://github.com/r-lib/tree-sitter-r/compare/99bf614d9d7e6ac9c7445fa7dc54a590fcdf3ce0...2097fa502efa21349d26af0ffee55d773015e481, most importantly:
- https://github.com/r-lib/tree-sitter-r/pull/134 to fix some panics we'd see in `Document::new()` when there are leading newlines before the first actual token in a document - I can't find a linked issue for it, but I added a test, and there are many on the tree-sitter-r side
- https://github.com/r-lib/tree-sitter-r/issues/141 to fix an edge case with if/else and newlines
- https://github.com/r-lib/tree-sitter-r/pull/145 to fix an edge case with functions and newlines

