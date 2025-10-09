# Add a workaround for an argument matching error

> <https://github.com/posit-dev/ark/pull/429>
> 
> * Author: @yutannihilation
> * State: MERGED
> * Labels: 

A workaround for https://github.com/posit-dev/positron/issues/3467.

I read https://github.com/posit-dev/positron/issues/3467#issuecomment-2158872927, and I don't think this is the proper fix to the problem, but it might be good as a temporary workaround if it will take some time to overhaul the signature-help method.

## @yutannihilation at 2024-07-08T12:10:44Z

Thanks for the review! I added `lsp::log_error!`.

> but not the `completionItem/resolve` ones in functions taking dots like `bar <- function(...) {}`.

It seems this occurs here (I'll paste the traceback in the original issue). I'm not yet sure if this is the same category of error that needs to be handled in this pull request, so I prefer a separate request at the moment.

https://github.com/posit-dev/ark/blob/8d44a5bc325312ede6e86d28ca5a57d82394b6e0/crates/ark/src/lsp/help.rs#L140-L144

(edit: `?bar` is an alias to `?plotmath`, which is not a help page about functions.)

## @lionel- at 2024-07-08T14:09:06Z

Thanks! These notifications are annoying.