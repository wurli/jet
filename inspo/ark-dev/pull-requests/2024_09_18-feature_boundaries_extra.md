# Distinguish whitespace and include syntax error messages

> <https://github.com/posit-dev/ark/pull/532>
>
> * Author: @lionel-
> * State: MERGED
> * Labels:

We now distinguish between complete inputs and pure whitespace. This way we don't need to send long streaks of empty lines and comments over the network for evaluation, since there is nothing to evaluate.

For the same reason invalid boundaries now carry an error message.

The data types have been reworked so that parse boundaries are now a simple vector of variant types.

Progress towards https://github.com/posit-dev/positron/issues/1326

