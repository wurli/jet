# Never flag `_` as an unknown `identifier`

> <https://github.com/posit-dev/ark/pull/804>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/3749
Addresses https://github.com/posit-dev/positron/issues/4102

tree-sitter parses this as an `identifier` node to keep things simple in the grammar, rather than parsing it as something like a `pipe_placeholder` within a pipe scope, which would be tough to do right. So we have to adapt to that here and always avoid flagging `_` as an unknown symbol.

