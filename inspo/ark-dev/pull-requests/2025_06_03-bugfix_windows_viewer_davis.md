# Localize special logic in `is_http_url()` helper

> <https://github.com/posit-dev/ark/pull/821>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Merges into https://github.com/posit-dev/ark/pull/818

We use `normalizePath()` with its intended meaning in a number of other places, so it feels a little confusing to me to have a shim of `normalize_path()` that we only use in these two places. With its current definition I'm not sure we'd want to use it everywhere in place of our current usage of `normalizePath()`.

Instead, how do you feel about an `is_http_url()` helper that we use in these two places, but still call `normalizePath()` our standard way?

I also was a little iffy about how we were comparing a known http url of `normalizedPath` against the `normalizedTempdir` - that comparison was really only meant for file paths. So I've tweaked that path to early exit when we recognize an http url.

---

I don't have my windows machine up and running, but I tested some urls on my mac just to make sure I didn't make any glaring typos.

