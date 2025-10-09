# Be willing to complete in the face of emptiness

> <https://github.com/posit-dev/ark/pull/772>
> 
> * Author: @jennybc
> * State: MERGED
> * Labels: 

Fixes #770

See the last "move" in https://github.com/posit-dev/ark/pull/805#discussion_r2089689821 for a concrete illustration of what this PR does. If the client asks for completions in any context where we aren't inside a node (i.e. we're just in the 'Program' node), we should basically send all composite completions, instead of no completions.

I've verified that the new tests fail on `main` and `bugfix/nearest-enclosing-node` for the reasons I expect.

