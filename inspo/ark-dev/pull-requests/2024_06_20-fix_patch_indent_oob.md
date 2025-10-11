# Hot patch `indent_edit()` for tree-sitter-r bug

> <https://github.com/posit-dev/ark/pull/413>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/3598

Again, this is really a tree-sitter-r issue solved by https://github.com/r-lib/tree-sitter-r/pull/126, the `program` node should really start at `(0, 0)`. i.e. all locations in a document should be contained within a node in the tree. When we sync with tree-sitter-r again we can and should remove this.


https://github.com/posit-dev/amalthea/assets/19150088/7fca191a-8c5c-458b-80e5-8591368025ae



