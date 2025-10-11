# Avoid evaluating promises during most completions

> <https://github.com/posit-dev/ark/pull/7>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

Addresses https://github.com/rstudio/positron/issues/586

Two major fixes:
- Promises are no longer ever forced, _except_ in namespace completions
- When we do force namespace completion promises, we now only do it exactly once. If `PRVALUE()` is set, we now extract that out and use it rather than calling `R_tryEvalSilent()` again.

---

Previously `completion_item_from_object()` was forcing any promise that came through it. This was problematic because it is the `CompletionItem` generator that pretty much every path goes through, so promises in the global environment (or eventually in a debug environment, when we support that) were also being forced when completions were being generated.

This PR ensures that `completion_item_from_object()` itself will never force a promise. If it sees an _unevaluated_ promise, then it will simply display a very generic item with a label of `"Promise"`.

This is what addresses https://github.com/rstudio/positron/issues/586

https://github.com/posit-dev/amalthea/assets/19150088/b6787b7f-0e6f-421e-a186-79ebc5b08561

---

That said, we currently do need to force promises when generating namespace completions, i.e. `dplyr::` or `dplyr:::`. There may be another way around this, I'm still not sure, but I couldn't come up with any good solution. We need to force them to be able to display the correct icon, and to be able to register a completion "command" which is especially important for functions. At the very least, I have now scoped the promise forcing to _only_ namespace completions. RStudio does a similar thing, so this is probably ok for at least the alpha release.

Notably we now don't force namespace `lazydata` completions, like `dplyr::starwars`. This is also similar to RStudio. It will display as a `"Promise"` until the user prints the object at least once, at which point a "real" `CompletionItem` can be generated for it the next time. We could improve on this in a future PR by creating a custom `CompletionItem` for `lazydata` objects that displayed something like `"Package object"` even if it is an unevaluated promise.

Extra note: By not forcing `lazydata` completions, this makes generating completions for `nycflights13::` much faster again! i.e. it resolves the slowdown I noted in: https://github.com/posit-dev/amalthea/pull/5#issue-17

https://github.com/posit-dev/amalthea/assets/19150088/63a1a510-7718-4178-b39d-2c9597edfb47



## @kevinushey at 2023-05-24T18:25:52Z

> When we do force namespace completion promises, we now only do it exactly once. If PRVALUE() is set, we now extract that out and use it rather than calling R_tryEvalSilent() again.

Wouldn't R's evaluation check whether a promise has already been evaluated and effectively do the same thing? (In other words, I think the semantics of this change are the same; it should hopefully just be a bit faster)

> That said, we currently do need to force promises when generating namespace completions, i.e. dplyr:: or dplyr:::. There may be another way around this, I'm still not sure, but I couldn't come up with any good solution.

I wonder if we can force a copy of the promise, so at least the existing promise in the namespace environment remains undisturbed. This might not be worth the effort though.

## @DavisVaughan at 2023-05-24T18:31:45Z

> I wonder if we can force a copy of the promise, so at least the existing promise in the namespace environment remains undisturbed. This might not be worth the effort though.

Yea I'm not really sure that is worth it. I don't think it is the the fact that the namespace environment is mutated that I'm particularly worried about. It is more about evaluating a promise that accidentally results in an active binding being triggered, or some other weird side effect (so even a copy would do that). I feel like we would have received bug reports about this in RStudio if it was really a big issue though.

## @DavisVaughan at 2023-05-24T18:33:01Z

> Wouldn't R's evaluation check whether a promise has already been evaluated and effectively do the same thing? (In other words, I think the semantics of this change are the same; it should hopefully just be a bit faster)

Yea I'm fairly certain `Rf_eval()` would just unwrap the promise, so yea this is likely just faster by avoiding all the "try" handling.

## @DavisVaughan at 2023-05-25T18:52:47Z

The latest change ensures that if a namespace has a promise that evaluates to an _error_ when we try and force it, then that original promise remains untouched. Previously it would be marked as "seen" (i.e. `PRSEEN()`) so if we tried to force it again on the next completion attempt it would throw a `"restarting interrupted promise evaluation"` warning.

Unfortunately there isn't a good API for cloning a promise, so we do something that is a little manual.

I used this code to bind an active binding that errors into the rlang namespace. We must use `delayedAssign()` to ensure that it is included in the namespace as a promise.

```r
ns <- asNamespace("rlang")

rlang::env_unlock(ns)
makeActiveBinding("foo_internal", function() stop("oh no"), ns)
delayedAssign("foo", foo_internal, eval.env = ns, assign.env = ns)

rlang::env_binding_unlock(ns, ".__NAMESPACE__.")
ns$.__NAMESPACE__.$exports$foo <- "foo"

rlang::env_binding_lock(ns, ".__NAMESPACE__.")
rlang::env_lock(ns)
```

You'll notice that `foo` is never suggested because of the error, but it is there in the namespace. And our generation of the completion options won't ever trigger the error or the promise evaluation warning.

https://github.com/posit-dev/amalthea/assets/19150088/3da56beb-a73a-4091-b387-20b251d7662a

