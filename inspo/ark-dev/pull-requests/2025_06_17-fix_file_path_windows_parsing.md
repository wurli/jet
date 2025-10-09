# Use safer `harp::parse_expr()` to decode file path strings

> <https://github.com/posit-dev/ark/pull/843>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/6584

Minimal reprex was to put `source(".\R\utils.R")` in the console and try and request completions with tab after the `utils.R`.

This string actually doesn't parse. It sees `\R` and complains that this is an unrecognized escape sequence.

This crashed ark on both mac and windows, so you don't need a windows machine to validate this fix, but it comes up on Windows because this is the string you get when you copy out of the windows file explorer (because they hate you).

Fixed by switching out the very old `r_string_decode()` for our more modern tooling of `parse_expr()`, which is safely wrapped in a `try_catch()`. Related to but unaffected by https://github.com/posit-dev/ark/pull/840. That PR will turn the actual error we get from an `harp::Error` into `ParseResult::SyntaxError`, but `parse_expr()` is insulated from that difference.

https://github.com/user-attachments/assets/c9ea13d9-d363-49ad-bead-975d7bfe5709




