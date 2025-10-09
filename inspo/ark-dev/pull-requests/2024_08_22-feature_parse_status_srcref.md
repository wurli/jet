# Add `SrcRef` type and create srcrefs with `parse_status()`

> <https://github.com/posit-dev/ark/pull/482>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Branched from #481.

Add `SrcRef` type and enable parsing with srcrefs in `parse_status()`. We now use this in `parse_exprs_with_srcrefs()` instead of calling back into the R-level `parse()`. Also adds a bunch of tests to this previously untested area.

Will be useful to write tools that detect expression boundaries in R code. Progress towards posit-dev/positron#1326

Edit: `srcref` is now a submodule of `parser`. I'll move `parse.rs` and `source.rs` to that submodule outside of this PR.

## @lionel- at 2024-08-26T14:15:31Z

@DavisVaughan Can you take another quick look please?

* I've moved the new files to a `parser` submodule. I plan to move `parse.rs` and `source.rs` there outside of this PR.

* The srcfile is no longer a parse option but a variant of input. I structured things this way so that srcfile is owned by the caller, which allows it to retrieve parse data.

* I renamed `line` to `line_virtual` and `line_parsed` to `line`.

* The line fields are now `[ )` ranges like the column fields. It's important not only for internal consistency but also because it's expected by methods of `std::ops::Range`.