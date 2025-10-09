# Use standard style in `handle_dotty_assignment()`

> <https://github.com/posit-dev/ark/pull/520>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

What do you think about this more standard style @kevinushey?

- Only create errors when something goes wrong (@DavisVaughan mentions this avoids performance issues with backtrace creation in some cases).

- Use standard constructors like `Ok()` and `Some()`.

-  If we're not interested in a value except for exiting, use explicit exits instead of `?`. I feel like the latter is the equivalent of doing `predicate() || return()` instead of the clearer `if (predicate()) return()`

