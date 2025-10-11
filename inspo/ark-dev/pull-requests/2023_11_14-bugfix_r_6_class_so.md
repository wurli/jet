# Prevent infinite recursion in the Variables thread

> <https://github.com/posit-dev/ark/pull/144>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

Addresses posit-dev/positron#1690

Environments can contain themselves either directly or indirectly:

```r
# Direct self-reference
x <- new.env(); x$x <- x

# Indirect self-reference
x <- list(globalenv())
```

Inspecting these self-referential structures is not a problem since inspection is on demand. The user just can't get to the bottom of the object. However, we currently recursively walk environments while creating `WorkspaceVariableDisplayValue` objects which causes stack overflows.

I first attempted to break the recursion by tracking seen environments (I left this attempt in the git history in case parts of it would be useful in the future). This was hard to get right and eventually I noticed that we were never actually using the result of the recursion when creating display values for environments, since we only show the names in that case. So simply removing the creation of display values for children in favour of pushing the names fixes the infinite recursion.

It would still be nice to keep track of seen environments at some point to provide hints that the user is exploring a self-referential structure. But given the complexity of this work, I think this should be done by the frontend so that backends don't have to implement it themselves. All they would need to do is return unique IDs for each node to the frontend. I can open an issue about this if you think such a feature would be useful to keep track of @jmcphers.

Working on this I noticed a failure mode for our recursive algorithm that I've documented in https://github.com/posit-dev/positron/issues/1817, and a potentially unsafe memory handling documented in https://github.com/posit-dev/positron/issues/1812.


