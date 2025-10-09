# Don't return file completions in string if trigger character is set

> <https://github.com/posit-dev/ark/pull/160>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/1884

Typing `"foo$"` triggers file completions because `$` is a completion trigger character.

To fix this, we now return an empty completion set inside strings if a trigger character like `$` is detected, as opposed to an explicit `TAB` hit (which causes a `None` trigger).

I took the easy route and added a `trigger` field to the document context that is passed around. This doesn't apply to Hover methods, which also get a document context, but that seems fine to me. In the future we could make it an enum if more trigger kinds are needed.

