# Fix perf issues with folding ranges

> <https://github.com/posit-dev/ark/pull/828>
> 
> * Author: @lionel-
> * State: MERGED
> * Labels: 

And fix a `tracing::error` message that showed up a ton of times in `ark:namespace:base.R`.

@kv9898 The algorithm was making a copy of the whole document split across n lines each time it materialised a line. Sorry I didn't see that while first reviewing.

Because of this, the folding range method took about 45s in the base namespace (try `debug(data.frame); data.frame()` to pop it up). Now back to 70ms.

Note that the folding range request takes a while to come in. I think the frontend also struggles with this large file.

