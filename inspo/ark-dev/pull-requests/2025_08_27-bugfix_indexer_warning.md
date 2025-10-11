# Key by `Url` instead of `Path` in workspace indexer

> <https://github.com/posit-dev/ark/pull/910>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/8790

The warnings only appear on Windows because the conversion to `Path` actually succeeds on Unixes. So we were calling `create() with non-file URIs on Unixes but not on Windows. To fix the warnings and make things more consistent, this PR:

- Now uses `Url` (wrapped in a new `FileId` struct) as key in our map of file to indexer. This is more general and simplifies a bunch of `Path` to `Url` conversions, avoiding by the same token potential warnings. The `FileId` wrapper is used for stronger internal type checking / self-documentation, but not exposed in the public API.

- Declines to index non-file URIs. This makes the check for `[rR]` files stronger.


### QA Notes

Typing in the console should no longer produce warnings in logs on Windows. I haven't been able to check as I don't have access to my Windows laptop right now.

