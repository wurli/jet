# Bump tree-sitter-r 2 - Add support for `StringContent` and `EscapeSequence` nodes

> <https://github.com/posit-dev/ark/pull/512>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Bumps us one more commit in tree-sitter-r to support the new `string_content` and `escape_sequence` children of a `string` node https://github.com/r-lib/tree-sitter-r/compare/bc8919d3c38b816652e1e2d1a1be037cf74364cb...9d1a68f8f239bc3749a481ac85e2163e24f6362c.

These are nice tree-sitter features but are a bit annoying for us. It means that `DocumentContext` will drill allllllll the way down to one of the following, rather than finding and stopping at just a `NodeType::String` like before:
- `Anonymous("'")` or `Anonymous("\"")`
- `StringContent`
- `EscapeSequence`

This means we have to walk back up the tree a bit when we need to work with or detect the `String` node. I think I've found all the places we do this.

I'm hoping this is the most annoying tree-sitter related tweak I'll have to do.

