# Don't derive `Ord` and `PartialOrd` for `RSymbol`

> <https://github.com/posit-dev/ark/pull/154>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

Compare the string data instead of the addresses.

Somehow the `sort()` test I added causes a crash without the new implementations but I don't know why.

There is no behaviour change in the variables pane, so I'm not sure whether the `sort()` you pointed me to has any impact @DavisVaughan.

