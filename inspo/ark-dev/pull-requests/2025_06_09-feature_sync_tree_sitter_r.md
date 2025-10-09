# Sync with tree-sitter-r

> <https://github.com/posit-dev/ark/pull/832>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

See also https://github.com/posit-dev/air/pull/357

Here are the meaningful changes we are pulling in:

- https://github.com/r-lib/tree-sitter-r/pull/171
- https://github.com/r-lib/tree-sitter-r/pull/172
- https://github.com/r-lib/tree-sitter-r/pull/174
- https://github.com/r-lib/tree-sitter-r/pull/175
- https://github.com/r-lib/tree-sitter-r/pull/157
- https://github.com/r-lib/tree-sitter-r/pull/154
- https://github.com/r-lib/tree-sitter-r/pull/153

All changes to our code are contained in the diagnostic module. I've added new tests for a number of these (in case we switch out the parser in the future, we don't want to regress). Each change is also its own commit.

The most annoying are https://github.com/r-lib/tree-sitter-r/pull/153 and https://github.com/r-lib/tree-sitter-r/pull/154. These both make tree-sitter-r _more_ accurate in terms of how it represents the R grammar, which is a good thing. `()` and `(1; 2)` are now syntax errors, and `fn(a b)` is also now a syntax error. This is great! But it means that tree-sitter just reports an `ERROR` at some unpredictable spot that _it_ gets to decide, and then it doesn't tell us anything about what kind of error it is (it can't do that and be general purpose at the same time). I've had to give up on giving "precise" errors in a few cases due to this, but I think that is okay. With a custom R Pratt parser, I think we should be able to do a lot better, because as we record parse errors we can report a _precise_ location and reason for the parse issue (i.e. missing parenthesis, or missing delimiter between arguments)

