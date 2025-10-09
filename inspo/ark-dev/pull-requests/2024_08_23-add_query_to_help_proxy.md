# Don't forget about query string in help proxy

> <https://github.com/posit-dev/ark/pull/484>
> 
> * Author: @juliasilge
> * State: MERGED
> * Labels: 

Addresses https://github.com/posit-dev/positron/issues/4320

Some of my "notes to self" in there are not 100% right but I did eventually get on the right path. The problem is that when we actually made the request, we didn't add back in the query string from the original URL, and that query string is used by R help for state management in its help server.

## QA Notes

- All requests for help that do _not_ use query strings, like `?lm`, should still work well
- Requests for help that _do_ use query strings, like `??cat`, should now also work well

