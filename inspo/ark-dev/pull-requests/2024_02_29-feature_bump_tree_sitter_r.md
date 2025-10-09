# Bump tree-sitter-r and pin tree-sitter to 0.21.0

> <https://github.com/posit-dev/ark/pull/259>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Previously we tracked dev tree-sitter, but they've released 0.21.0 since then, and now their dev branch is pretty unstable as they work towards a 1.0.0 release. So let's just pin to 0.21.0. We have to make 1 small change where `set_language()` now takes a reference to a language object.

The main reason to bump any of this is to get this tree-sitter-r PR in here
https://github.com/r-lib/tree-sitter-r/pull/73

I ran `cargo update -p tree-sitter-r` which updated the lock file to point at the newest `next` commit

