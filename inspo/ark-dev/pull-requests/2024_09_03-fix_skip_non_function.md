# Recognize non-function pages in `RHtmlHelp` and split it into topic vs function

> <https://github.com/posit-dev/ark/pull/500>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Closes https://github.com/posit-dev/ark/pull/431 - alternate approach

Addresses https://github.com/posit-dev/positron/issues/3467#issuecomment-2213963654 (meaning we can close the issue after this)

Example from https://github.com/posit-dev/ark/pull/431:

```r
bar <- function(...) {}

bar()
#   ^
#   get errors when moving a cursor to here
```

When you get to `bar(<here>)`, we try to look up help documentation for `?bar`. That actually gets mapped to `?plotmath`, a pure topic page (i.e. no functions on there). This was causing our `RHtmlHelp::parameters()` function to freak out, because it was only to be used on function pages.

`RHtmlHelp` is a generic help engine, so it should be able to look up non-function arbitrary topics. And in fact we do this for package pages, i.e. `"dplyr-package"` in `resolve_package_completion_item()`, so we can't just throw out any help page that doesn't look like function help (which was the #431 approach).

Instead, I've split `RHtmlHelp` into:
- `from_topic()`, which sets `function = false` internally
- `from_function()`, which sets `function = true` internally

`from_function()` also returns `None` if a new `is_function()` internal helper returns `false` on the found HTML. It uses the approach from #431 which was to detect a `Usage` section, which should only apply to functions.

We now use `from_function()` everywhere except `resolve_package_completion_item()`, but I see us potentially using `from_topic()` more in the future too.

The new internal `function: bool` flag is useful because it allows us to error immediately in `RHtmlHelp::parameter()` and `RHtmlHelp::parameters()` if we don't give it function related help. We could try to encode this into the type system but that seemed like overkill for now.

---

I've also introduced snapshot testing through the insta crate in this PR. It was useful for catching an expected error message, and I think we are going to want to use it elsewhere too.

