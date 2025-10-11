# R: Don't inline arguments in function calls to prevent overwhelming information in error messages

> <https://github.com/posit-dev/ark/issues/695>
>
> * Author: @lionel-
> * State: OPEN
> * Labels:

When we make a function call from Rust to R in Ark, we currently inline arguments in our calls. This can result in overwhelming display of information when printing the call, e.g. this example from @jmcphers:

```
exiting from: .ps.filter_rows(table = list(mpg = c(21, 21, 22.8, 21.4, 18.7,
18.1, 14.3, 24.4, 22.8, 19.2, 17.8, 16.4, 17.3, 15.2, 10.4, 10.4,
14.7, 32.4, 30.4, 33.9, 21.5, 15.5, 15.2, 13.3, 19.2, 27.3, 26,
30.4, 15.8, 19.7, 15, 21.4), cyl = c(6, 6, 4, 6, 8, 6, 8, 4,
4, 6, 6, 8, 8, 8, 8, 8, 8, 4, 4, 4, 4, 8, 8, 8, 8, 4, 4, 4, 8,
6, 8, 4), disp = c(160, 160, 108, 258, 360, 225, 360, 146.7,
140.8, 167.6, 167.6, 275.8, 275.8, 275.8, 472, 460, 440, 78.7,
75.7, 71.1, 120.1, 318, 304, 350, 400, 79, 120.3, 95.1, 351,
145, 301, 121), hp = c(110, 110, 93, 110, 175, 105, 245, 62,
95, 123, 123, 180, 180, 180, 205, 215, 230, 66, 52, 65, 97, 150,
```

In particular these overly exhaustive messages can make it all the way to the frontend in error notification popups. This happens when an unexpected error happens on one of our R methods. The error is formatted with the offending call and propagated all the way back to the frontend as an RPC error, which we display as notification: https://github.com/posit-dev/positron/issues/2195

To make these messages less distracting and more to the point, we could change our `call()` method for `RFunction` to bind arguments in an environment (a child of the target environment for the call evaluation) so that the function call would contain symbols instead of inlined objects. The evaluation semantics would not be 100% identical when the function call performs side effects in the evaluation environment. I don't think we do that but this is worth a quick check through our call sites.

## @DavisVaughan at 2024-05-13T17:44:17Z

In https://github.com/posit-dev/positron/issues/3117 we saw a case where the inlined object was >512mb (a data frame with nested tibbles, quite complicated).

This actually caused:
- The whole Console to hang for 30-40 seconds, before giving control back
- The LSP to crash, because Node errored and refused to create a string of >512mb, and that took down the Client side of the LSP causing it to disconnect from Ark's server side

I've fixed the underlying issue causing https://github.com/posit-dev/positron/issues/3117, but we should still fix this too to prevent the LSP from getting taken down, so I'm going to bump this to RC

## @lionel- at 2024-05-14T14:09:49Z

In addition to the changes proposed in this issue, we might also want to check for excessively long error messages at the interop boundary because we are not in full control of captured error calls. Also need to check whether captured backtraces are trimmed.

## @DavisVaughan at 2024-07-17T20:43:44Z

https://github.com/posit-dev/positron/issues/4008 is another case of this issue
