# Custom `statement_range()` LSP message

> <https://github.com/posit-dev/ark/pull/85>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Joint with https://github.com/posit-dev/positron/pull/1227 - see there for full details

`goto_first_child_for_point()` doesn't work as expected, see https://github.com/tree-sitter/tree-sitter/issues/2012, so I'm using a Rust implementation of the patch they used in Emacs https://github.com/tree-sitter/tree-sitter/issues/2012#issuecomment-1385623880

