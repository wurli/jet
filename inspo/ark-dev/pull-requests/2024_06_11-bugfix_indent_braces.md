# Fix indentation issues with braces

> <https://github.com/posit-dev/ark/pull/393>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/3475
Addresses https://github.com/posit-dev/positron/issues/3484

- Import snapshots from https://github.com/emacs-ess/ESS/blob/master/test/styles/RStudio-.R to assess behaviour of the indenter. Note that currently only pipelines and brace children are reindented using Ark, not other lines.

- Fix a bunch of indentation issues to make the above snapshots look okay. See new test cases on the Rust side.

- Fix indentation of the two issues linked above. The fix for the closing delimiter issue consists in recursing into `indent_line()` for the line of that delimiter and appending the edits to the return value.

