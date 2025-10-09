# Support roxygen2 `@examples` execution

> <https://github.com/posit-dev/ark/pull/126>
> 
> * Author: @DavisVaughan
> * State: MERGED
> * Labels: 

Closes https://github.com/posit-dev/amalthea/pull/122 (supersedes it)

Joint work with @juliasilge 

Related to https://github.com/posit-dev/positron/pull/1627 which changes `StatementRangeProvider` so it returns both a range and optionally code to execute from that range, if it differs from the document contents. The Python implementation is in posit-dev/positron-python#233 and we're doing this mostly for https://github.com/posit-dev/positron/issues/1410. 

Merging this will temporarily break main amalthea if you also have main positron. There will be a very short period where you'd need main amalthea and https://github.com/posit-dev/positron/pull/1627 together, but we will keep that brief.

---

With this PR, code within an `@examples` block is sent _one line at a time_ back over to Positron (i.e. a "dumb" version of the statement range provider), which is also what RStudio does, and it hasn't felt that awful.

When we are sending over code in `@examples`, we strip the leading `#'` and an optional single whitespace after that and send that over in the new optional `code` slot of the `StatementRangeResponse`

Notably, when we are _outside_ of `@examples` we still send the roxygen comments. We send the entire current line as a comment, so you can mash `CMD + Enter` to step your way down to the `@examples` section if you happen to click in the wrong place (I don't expect this to be common at all).

