# Don't normalize HTTP URLs for the Viewer

> <https://github.com/posit-dev/ark/pull/818>
>
> * Author: @jmcphers
> * State: MERGED
> * Labels:

This change addresses an issue that keeps URLs from being loaded into the Viewer on Windows. The issue is that we call `normalizePath` on everything that gets loaded into the Viewer, which is great for file paths but unsurprisingly doesn't work on URLs.

The fix here is to make our own wrapper for `normalizePath` that passes URLs through safely. This new function isn't vectorized (unlike the original `normalizePath`) but shouldn't need to be since the viewers accept only a single URL or path.

Addresses https://github.com/posit-dev/positron/issues/4843

## @jmcphers at 2025-06-03T15:48:17Z

Yeah, I think that's better!
