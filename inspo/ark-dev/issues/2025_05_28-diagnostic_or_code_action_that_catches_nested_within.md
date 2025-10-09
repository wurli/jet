# Diagnostic or Code Action that catches `#` nested within `#'`

> <https://github.com/posit-dev/ark/issues/810>
> 
> * Author: @DavisVaughan
> * State: OPEN
> * Labels: 

From @EmilHvitfeldt 

```r
#' Title
#'
# @param ...
#'
#' @returns
#'
#' @export
```

Notice the `# @param` line missing the `#'`.

We can probably detect this by looking for `#` sandwiched directly between `#'`?

## @lionel- at 2025-05-27T07:50:42Z

I wonder if Air should even treat this as a syntax/parser error?
I.e. would it make sense to augment the parser with new classes of errors?

Probably not because that would prevent formatting the file, unless we can distinguish between base errors and extended errors. Probably simpler to make it a lint.

Sorry just thinking out loud here 😄 

Could also add the missing `'` via the formatter.

## @etiennebacher at 2025-05-28T13:18:24Z

> Could also add the missing ' via the formatter.

Hi, I just want to mention that having `# @param ...` is sometimes desired, for instance when working on bindings and some arguments of a function should be implemented eventually but we don't have them yet (usually we have a `# TODO` above).

Of course this is quite a corner case so it might be an instance of https://xkcd.com/1172/, but having this being automatically fixed by Air would be annoying (highlighting this as a syntax error would be enough IMO). Anyway, just my 2 cents.