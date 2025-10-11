# Turn on workspace setting to trim trailing whitespace

> <https://github.com/posit-dev/ark/pull/46>
>
> * Author: @DavisVaughan
> * State: MERGED
> * Labels:

And turn on the setting to show whitespace differences in the diff viewer.

@lionel- noticed that `errors.R` had trailing whitespace 😱. I don't think we ever want this, so let's make it a Workspace setting.

I imagine other people may already have it as a User setting, so that may be why we haven't seen this before.

