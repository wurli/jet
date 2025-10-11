# Avoid dispatching on `names` when listing environment bindings.

> <https://github.com/posit-dev/ark/pull/364>
>
> * Author: @dfalbel
> * State: MERGED
> * Labels:

Addresses https://github.com/posit-dev/positron/issues/3229.


## @DavisVaughan at 2024-06-06T21:23:45Z

@dfalbel looks like you can update now if you want

## @dfalbel at 2024-06-07T13:47:31Z

Just updated the PR.

- I tried to use `EnvironmentFilter::default()` when the implementation didn't really care about the filter state and specify firmly when we required one type or the other.
- Since the iter already records the names, we don't need it to record the filter type as it would be already filtered.



## @dfalbel at 2024-06-12T15:00:24Z

> Some comments and also I think the filter should be forwarded to iter()?

The iter takes a vector of names that is then used to recover the bindings, so the filter is implicitly done at that point. Do you think that's enough, or should we explicitly list all bindings and then filter based on the filter argument?
