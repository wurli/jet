# Support namespace operator nodes in `help_topic()`

> <https://github.com/posit-dev/ark/pull/339>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/2985

We mostly had everything hooked up right, as `.ps.help.showHelpTopic()` already knows how to split `pkg::fun()` into `pkg` and `fun`. The problem is that our LSP support for F1, i.e. `help_topic()`, was only sending back the identifier `fun`, and was not aware of the fact that it could be part of a `::` or `:::` node, in which case it should be sending back `pkg::fun`. Without the `pkg` prefix, we end up calling `help(fun)`, which works if `pkg` is loaded, but fails if it isn't. `help(fun, pkg = pkg)` works either way.

I've fixed that here, and added tests for that.

I've also made `.ps.help.showHelpTopic()` a little smarter by teaching it how to handle `pkg:::fun()` too. That way, if I've typed `dplyr:::across()` (see `:::`) into my editor and I hit F1 on that, Positron is still smart enough to look up the help docs for `across()`. I think it may also be possible to have help docs for unexported objects, so this may allow that to "just work". Anyways, I've added two new tests for that half of the conversation as well - testing that we can show the help for topics `"utils::find"` and `"utils:::find"`.

